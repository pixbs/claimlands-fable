//! The cliff, surf and field strips: small sprites drawn per texel so they are true pixel art,
//! prototype section 2 and the field texture of section 4b.

use cl_model::{RgbaImage, hex_rgb};
use cl_noise::hash2;
use cl_noise::js::{round, sin};

use crate::palette::{CLIFF_ROWS, FIELD_CROPS, FOAM_ROWS};

/// Width of the cliff strip; it repeats along an edge.
pub const CLIFF_W: u32 = 24;
/// Height of the cliff strip: a wall is exactly four texture pixels per elevation step.
pub const CLIFF_H: u32 = 4;
/// How far the coast slope runs out over the water, in texture pixels.
pub const BEACH_PX: f64 = 3.0;
/// Width of the surf band beyond the slope, in texture pixels.
pub const FOAM_PX: f64 = 7.0;
/// Width of the surf sprite.
pub const FOAM_W: u32 = 24;
/// Height of one surf frame: tracks `FOAM_PX` so the texels stay square.
pub const FOAM_H: u32 = 7;
/// Frames in the surf sheet, stacked vertically.
pub const FOAM_FRAMES: u32 = 8;
/// World pixels per furrow cycle: one dark line, four plain.
pub const FURROW_PX: u32 = 5;
/// Rows of the field strip: the flat top and the side, per crop.
pub const FIELD_ROWS: u32 = FIELD_CROPS.len() as u32 * 2;

/// How a texture axis wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    /// Clamp to edge.
    Clamp,
    /// Repeat.
    Repeat,
}

/// An image plus the sampler settings the prototype's `pixelTexture` sets: nearest filtering, no
/// mipmaps, and the given wrap modes.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// Texels.
    pub image: RgbaImage,
    /// Horizontal wrap.
    pub wrap_s: Wrap,
    /// Vertical wrap.
    pub wrap_t: Wrap,
    /// UV repeat factors (`texture.repeat` in three.js).
    pub repeat: [f64; 2],
}

fn put(img: &mut RgbaImage, x: u32, y: u32, c: [u8; 3]) {
    img.put(x, y, [c[0], c[1], c[2], 255]);
}

/// The cliff face. Repeats horizontally along an edge and once vertically per elevation level.
/// Some texels are nudged to a neighbouring row so the strata are not ruler-straight.
pub fn make_cliff_texture() -> Texture {
    let rows: Vec<[u8; 3]> = CLIFF_ROWS.iter().map(|h| hex_rgb(h)).collect();
    let mut img = RgbaImage::new(CLIFF_W, CLIFF_H);
    for y in 0..CLIFF_H {
        for x in 0..CLIFF_W {
            let mut c = rows[y as usize];
            let n = hash2(f64::from(x) * 1.7, f64::from(y) * 3.1);
            if y > 0 && n > 0.82 {
                c = rows[y as usize - 1];
            } else if y < CLIFF_H - 1 && n < 0.16 {
                c = rows[y as usize + 1];
            }
            put(&mut img, x, y, c);
        }
    }
    Texture {
        image: img,
        wrap_s: Wrap::Repeat,
        wrap_t: Wrap::Repeat,
        repeat: [1.0, 1.0],
    }
}

/// Surf: `FOAM_FRAMES` frames stacked vertically; the render loop animates the sheet by moving
/// the texture offset. The shape along the shore is fixed and only its amplitude breathes, so the
/// crest never slides sideways. Row 0 of a frame is the shore side. Frame `f` occupies canvas rows
/// from `FOAM_H * (FOAM_FRAMES - 1 - f)` because the sheet is sampled with `flipY`.
pub fn make_foam_texture() -> Texture {
    let rows: Vec<[u8; 3]> = FOAM_ROWS.iter().map(|h| hex_rgb(h)).collect();
    let mut img = RgbaImage::new(FOAM_W, FOAM_H * FOAM_FRAMES);
    for f in 0..FOAM_FRAMES {
        let swell = 0.28
            + 0.72
                * (0.5
                    + 0.5
                        * sin(f64::from(f) / f64::from(FOAM_FRAMES) * std::f64::consts::PI * 2.0));
        for x in 0..FOAM_W {
            let xf = f64::from(x);
            let shape = 1.15
                + 1.25 * sin(xf * 0.55)
                + 0.75 * sin(xf * 0.23 + 2.1)
                + 0.45 * hash2(xf * 1.3, 7.7);
            let crest = shape * swell * (f64::from(FOAM_H) / 4.0);
            for j in 0..FOAM_H {
                let cy = FOAM_H * (FOAM_FRAMES - 1 - f) + j;
                if f64::from(j) + 0.5 >= crest {
                    continue; // transparent: the image starts cleared
                }
                let idx = ((j * rows.len() as u32) / FOAM_H) as usize;
                put(&mut img, x, cy, rows[idx.min(rows.len() - 1)]);
            }
        }
    }
    Texture {
        image: img,
        wrap_s: Wrap::Repeat,
        wrap_t: Wrap::Clamp,
        repeat: [1.0, 1.0 / f64::from(FOAM_FRAMES)],
    }
}

/// Canvas row of the field strip to a `v` coordinate, `flipY` aware.
pub fn field_row_v(row: u32) -> f64 {
    (f64::from(FIELD_ROWS) - 0.5 - f64::from(row)) / f64::from(FIELD_ROWS)
}

/// One column per furrow cycle, two rows per crop: the flat top, then the side. Five texels wide
/// with nearest sampling means one texel is exactly one world pixel.
pub fn make_field_texture() -> Texture {
    let mut img = RgbaImage::new(FURROW_PX, FIELD_ROWS);
    for (i, crop) in FIELD_CROPS.iter().enumerate() {
        let base = hex_rgb(crop.base);
        let furrow = hex_rgb(crop.furrow);
        let edge = hex_rgb(crop.edge);
        let edge_line = edge.map(|v| round(f64::from(v) * 0.84) as u8);
        let i = i as u32;
        for x in 0..FURROW_PX {
            put(&mut img, x, 2 * i, if x == 0 { furrow } else { base });
            put(
                &mut img,
                x,
                2 * i + 1,
                if x == 0 { edge_line } else { edge },
            );
        }
    }
    Texture {
        image: img,
        wrap_s: Wrap::Repeat,
        wrap_t: Wrap::Clamp,
        repeat: [1.0, 1.0],
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_wraps() {
        let c = make_cliff_texture();
        assert_eq!((c.image.width, c.image.height), (24, 4));
        let f = make_foam_texture();
        assert_eq!((f.image.width, f.image.height), (24, 56));
        assert_eq!(f.wrap_t, Wrap::Clamp);
        assert_eq!(f.repeat, [1.0, 0.125]);
        let s = make_field_texture();
        assert_eq!((s.image.width, s.image.height), (5, 8));
        assert_eq!(s.image.get(0, 0)[..3], hex_rgb(FIELD_CROPS[0].furrow));
        assert_eq!(field_row_v(0), 7.5 / 8.0);
    }
}
