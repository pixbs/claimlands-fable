//! The cloud sky: three deck textures on one equal-area cylinder, prototype `makeCloudSky`.
//!
//! The decks have to drift while their shells stay put, so the pattern moves as a texture offset
//! rather than being re-rasterised. That only equals a rotation of the sky if the mapping is
//! cylindrical about the drift axis, and a texture stretched over a cylinder smears at the poles —
//! so each texel is mapped back to the direction it actually covers and a 3D field is sampled
//! there. The latitude axis tracks the sine of the angle, not the angle, which makes every texel
//! cover the same solid angle: over-sampled at the poles, undistorted everywhere.
//!
//! Equal area is also what lets a deck be stated as a share of sky. Thresholds spaced evenly up
//! the field are spaced very unevenly in area, so each deck's cut is taken as a quantile of the
//! texels themselves; because every sample counts the same, a quantile *is* a share of sky.

use cl_model::{Filter, RgbaImage, Texture, Wrap};
use cl_noise::fbm3;
use cl_noise::js::{cos, round, sin};

/// One cloud deck: the tint the shell is drawn in, and the share of sky it covers. Outer deck
/// first; each sits strictly inside the one below, so the stack terraces inward.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudDeck {
    /// Tint of the deck's shell. Not in the texture, which is white throughout.
    pub tone: &'static str,
    /// Share of the sky the deck covers, as a fraction in `0..1`.
    pub cover: f64,
}

/// The three decks, outermost first. Coverage rather than a field threshold: spacing thresholds
/// evenly up the noise gave the top deck one per cent of the sky, a speck instead of a summit.
pub const CLOUD_DECKS: [CloudDeck; 3] = [
    CloudDeck {
        tone: "#b9d4e8",
        cover: 0.38,
    },
    CloudDeck {
        tone: "#dceaf5",
        cover: 0.20,
    },
    CloudDeck {
        tone: "#ffffff",
        cover: 0.09,
    },
];

/// Octaves of the cloud field.
pub const CLOUD_OCT: u32 = 3;
/// Base frequency of the cloud field.
pub const CLOUD_F0: f64 = 3.4;
/// Width of the sky texture in texels. Fixed rather than derived from the world pixel: one sky is
/// shared by every world size, so deriving it from whichever size asked first made the resolution
/// depend on how fast the player moved the slider. 928 puts one texel on one world pixel at the
/// default size. A multiple of four, so the dither wraps without a seam where the sky joins itself.
pub const CLOUD_TEX_W: u32 = 928;

/// Ordered 4×4 dither, in the prototype's order: `BAYER4[(y & 3) * 4 + (x & 3)]` is the rank of a
/// texel.
pub const BAYER4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
/// Ranks in the dither.
pub const DITHER_RANKS: u32 = 16;
/// Alpha below which the cloud shader discards a texel. Stored alpha is scaled so that rank 0
/// lands at `DITHER_FLOOR * 2 * DITHER_RANKS`, which is why the floor has to sit at or under
/// `1/32`.
pub const DITHER_FLOOR: f64 = 0.030;

/// Texels the quantile probe subsamples the field down to. Sorting all `W * H` of them three times
/// buys nothing: the cut is a quantile, and at this many samples it lands on the same texel value.
const PROBE_MAX: usize = 40000;

/// The cloud sky: one texture per deck on the shared equal-area grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Sky {
    /// Deck textures, outermost first, parallel to [`CLOUD_DECKS`].
    pub decks: [Texture; 3],
    /// Width of every deck in texels.
    pub width: u32,
    /// Height of every deck in texels.
    pub height: u32,
}

/// Height of the sky for a given width: the full sphere is `2π` wide by `2` tall, so the aspect is
/// `π`. Rounded to a multiple of four so the 4×4 dither wraps on the latitude axis too.
fn sky_height(width: u32) -> u32 {
    4 * round(f64::from(width) / std::f64::consts::PI / 4.0) as u32
}

/// Stored alpha per Bayer rank. A texel survives while `seeThrough > (rank + 0.5) / 16`, so the
/// stored value is the *reciprocal* of the rank and the surviving fraction equals `seeThrough`
/// exactly. Two things fall out: untouched sky is completely solid, because at `seeThrough == 1`
/// every rank clears the floor and a cloud left alone shows no dither at all; and the dissolve is
/// linear, where storing the rank straight held coverage near 1 until it fell off a cliff.
fn rank_alpha() -> [u8; 16] {
    BAYER4.map(|b| {
        round(255.0 * (DITHER_FLOOR * f64::from(DITHER_RANKS) / (f64::from(b) + 0.5)).min(1.0))
            as u8
    })
}

/// Three cloud decks from one seed. Every deck is white; the tint lives on the material, and the
/// alpha channel carries the dither rank so the shader can dissolve a deck by lowering opacity.
pub fn make_cloud_sky(seed: f64) -> Sky {
    let (w, h) = (CLOUD_TEX_W, sky_height(CLOUD_TEX_W));

    let mut field = vec![0f32; (w as usize) * (h as usize)];
    for j in 0..h {
        let sy = 2.0 * (f64::from(j) + 0.5) / f64::from(h) - 1.0;
        let cy = (1.0 - sy * sy).max(0.0).sqrt();
        for i in 0..w {
            let lam = (f64::from(i) + 0.5) / f64::from(w) * 2.0 * std::f64::consts::PI;
            let p = [cy * cos(lam), sy, cy * sin(lam)];
            field[(j as usize) * (w as usize) + i as usize] =
                fbm3(p, seed, CLOUD_OCT, CLOUD_F0) as f32;
        }
    }

    let stride = (field.len() / PROBE_MAX).max(1);
    let mut probe: Vec<f32> = field.iter().copied().step_by(stride).collect();
    probe.sort_by(|a, b| a.total_cmp(b));

    let alpha = rank_alpha();
    let decks = CLOUD_DECKS.map(|deck| {
        let at = (probe.len() as f64 * (1.0 - deck.cover)).floor() as usize;
        let cut = probe[at.min(probe.len() - 1)];
        let mut image = RgbaImage::new(w, h);
        for j in 0..h {
            for i in 0..w {
                let lit = field[(j as usize) * (w as usize) + i as usize] >= cut;
                let a = if lit {
                    alpha[((j & 3) * 4 + (i & 3)) as usize]
                } else {
                    0
                };
                image.put(i, j, [255, 255, 255, a]);
            }
        }
        Texture {
            image,
            // Drift wraps round the planet; the latitude axis must not, or the poles would bleed.
            wrap_s: Wrap::Repeat,
            wrap_t: Wrap::Clamp,
            filter: Filter::Nearest,
            repeat: [1.0, 1.0],
        }
    });

    Sky {
        decks,
        width: w,
        height: h,
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn sky_is_an_equal_area_cylinder() {
        // Aspect π, and both axes a multiple of four so the dither wraps on either.
        assert_eq!(sky_height(CLOUD_TEX_W), 296);
        assert_eq!(CLOUD_TEX_W % 4, 0);
        assert_eq!(sky_height(CLOUD_TEX_W) % 4, 0);
    }

    #[test]
    fn rank_alpha_is_the_reciprocal_of_the_rank() {
        let a = rank_alpha();
        // Rank 0 survives the deepest fade, so it stores the highest alpha; rank 15 the lowest.
        assert_eq!(a[0], 245);
        assert_eq!(a[BAYER4.iter().position(|&b| b == 15).unwrap()], 8);
        // Every rank clears the discard floor at full opacity: untouched sky shows no dither.
        for &v in &a {
            assert!(f64::from(v) / 255.0 > DITHER_FLOOR);
        }
    }

    #[test]
    fn decks_terrace_inward() {
        let sky = make_cloud_sky(3521.0);
        let covered = |t: &Texture| {
            t.image
                .data
                .iter()
                .skip(3)
                .step_by(4)
                .filter(|&&a| a > 0)
                .count()
        };
        let counts: Vec<usize> = sky.decks.iter().map(covered).collect();
        assert!(counts[0] > counts[1] && counts[1] > counts[2], "{counts:?}");
        // A quantile of an equal-area grid is a share of sky, to within one probe step.
        let texels = (sky.width as usize) * (sky.height as usize);
        for (deck, &n) in CLOUD_DECKS.iter().zip(&counts) {
            let share = n as f64 / texels as f64;
            assert!(
                (share - deck.cover).abs() < 0.005,
                "{share} vs {}",
                deck.cover
            );
        }
    }
}
