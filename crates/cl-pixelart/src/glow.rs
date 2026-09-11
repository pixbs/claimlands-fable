//! The halo: a radial gradient in the atmosphere's own colour, prototype `makeGlow`.
//!
//! A flat quad parked behind the planet so the air does not simply stop at the rim. It lives in the
//! scene rather than on the planet, so it neither spins nor tilts, and because the camera only ever
//! sits on `+z` looking at the origin a plane in `xy` always faces it squarely — no billboarding.
//!
//! The quad is scaled so its edge lands at [`GLOW_OUT`] atmosphere radii, which puts the rim itself
//! at `1 / GLOW_OUT` of the way out. The gradient is flat inside that and falls away outside it, so
//! none of it is spent under the planet and the air, where it could not be seen anyway.

use cl_model::{Filter, RgbaImage, Texture, Wrap, hex_rgb};
use cl_noise::js::{hypot2, pow, round};

use crate::palette::AIR_COLOR;

/// Side of the halo image in texels. Written inline in the prototype, so `fixtures/constants.json`
/// does not carry it. It is sampled linearly and always magnified, so the size only has to be
/// enough that the gradient does not band.
pub const GLOW_PX: u32 = 128;
/// How far out the halo reaches, in atmosphere radii.
pub const GLOW_OUT: f64 = 1.16;
/// How far behind the planet the quad is parked, in atmosphere radii. Far enough to clear the whole
/// air shell in depth.
pub const GLOW_BACK: f64 = 1.30;
/// Alpha of the flat centre, and the value the falloff starts from.
pub const GLOW_MAX: f64 = 0.42;
/// Exponent of the falloff outside the rim. Above 1 it leaves the rim quickly and lingers faint
/// further out, which is how air reads. Inline in the prototype, like [`GLOW_PX`].
pub const GLOW_FALLOFF: f64 = 1.7;

/// The halo texture: [`AIR_COLOR`] throughout, the gradient carried entirely by alpha.
///
/// Linear filtering rather than nearest — this is a glow, not pixel art, and nearest would band it
/// into visible rings.
pub fn make_glow() -> Texture {
    let c = hex_rgb(AIR_COLOR);
    let n = f64::from(GLOW_PX);
    let peak = 1.0 / GLOW_OUT;
    let mut image = RgbaImage::new(GLOW_PX, GLOW_PX);
    for y in 0..GLOW_PX {
        let dy = (f64::from(y) + 0.5) / n * 2.0 - 1.0;
        for x in 0..GLOW_PX {
            let dx = (f64::from(x) + 0.5) / n * 2.0 - 1.0;
            let r = hypot2(dx, dy);
            let a = if r <= peak {
                GLOW_MAX
            } else if r < 1.0 {
                GLOW_MAX * pow((1.0 - r) / (1.0 - peak), GLOW_FALLOFF)
            } else {
                0.0
            };
            image.put(x, y, [c[0], c[1], c[2], round(a * 255.0) as u8]);
        }
    }
    Texture {
        image,
        // The prototype never sets a wrap on this one, so it keeps three.js' clamped default; the
        // quad's UVs never leave `0..1` anyway.
        wrap_s: Wrap::Clamp,
        wrap_t: Wrap::Clamp,
        filter: Filter::Linear,
        repeat: [1.0, 1.0],
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn the_centre_is_flat_and_the_corners_are_empty() {
        let tex = make_glow();
        let air = hex_rgb(AIR_COLOR);
        let mid = GLOW_PX / 2;
        let peak_alpha = round(GLOW_MAX * 255.0) as u8;
        assert_eq!(
            tex.image.get(mid, mid),
            [air[0], air[1], air[2], peak_alpha]
        );
        // A corner is at radius √2, past the edge of the gradient: transparent, but still air.
        assert_eq!(tex.image.get(0, 0)[3], 0);
        assert_eq!(&tex.image.get(0, 0)[..3], &air[..]);
    }

    #[test]
    fn alpha_falls_away_from_the_rim_and_never_rises() {
        let tex = make_glow();
        let mid = GLOW_PX / 2;
        // Walking out along the middle row from the centre, alpha never increases.
        let mut last = 255u8;
        for x in mid..GLOW_PX {
            let a = tex.image.get(x, mid)[3];
            assert!(a <= last, "alpha rose at x={x}: {a} after {last}");
            last = a;
        }
        // Flat all the way out to the rim.
        let rim = (GLOW_PX / 2) + (f64::from(GLOW_PX) / 2.0 / GLOW_OUT) as u32 - 1;
        assert_eq!(tex.image.get(rim, mid)[3], round(GLOW_MAX * 255.0) as u8);
        // The middle row's last texel is at r ≈ 0.992 — still inside the gradient, so it is faint
        // rather than empty. Alpha reaches exactly zero only outside the unit circle, which on this
        // row is past the image. The corners are the ones that get there.
        assert!(tex.image.get(GLOW_PX - 1, mid)[3] <= 2);
        assert_eq!(tex.image.get(GLOW_PX - 1, 0)[3], 0, "the corner is outside");
    }

    #[test]
    fn the_gradient_is_radially_symmetric() {
        let tex = make_glow();
        // The sampling grid is symmetric about the centre, so opposite texels must agree exactly.
        for i in 0..GLOW_PX {
            let j = GLOW_PX - 1 - i;
            assert_eq!(tex.image.get(i, 7), tex.image.get(j, 7), "mirrored at {i}");
            assert_eq!(tex.image.get(7, i), tex.image.get(7, j), "mirrored at {i}");
            // And the two axes carry the same profile.
            assert_eq!(
                tex.image.get(i, 7),
                tex.image.get(7, i),
                "transposed at {i}"
            );
        }
    }
}
