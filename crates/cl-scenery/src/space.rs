//! The backdrop, prototype section SPACE (`buildSpace`, `stepStars`): a vignette plane and a field
//! of stars, drawn as their own orthographic pass before the planet rather than as a CSS gradient
//! behind the canvas.
//!
//! The canvas renders at `1 / pixel_scale` and is upscaled with nearest sampling, so one texel of
//! this pass is one world pixel on screen, exactly like the ground. A backdrop behind that grid
//! would stay smooth while everything in front of it stayed chunky, so the backdrop is geometry in
//! the same low-resolution target.
//!
//! The vignette is a coarse grid of vertex colours and nothing more: the low render size does the
//! quantising, and that is what gives the falloff its banded pixel-art edge.
//!
//! Stars are quads snapped to the render-pixel lattice rather than pixels painted into a texture,
//! so a flicker step costs a rewrite of a few hundred vertex colours instead of a fresh upload of
//! the whole backdrop.

use cl_model::{MeshData, hex_rgb};
use cl_noise::hash3i;
use cl_noise::js::{hypot2, pow, round};

/// Centre of the sky, where the vignette is lightest.
pub const SKY_CORE: &str = "#0f0c26";
/// Rim of the sky, and the colour the frame is cleared to behind this pass.
pub const SKY_RIM: &str = "#030209";
/// Stars per render pixel.
pub const STAR_DENSITY: f64 = 0.00085;
/// Share of the stars that are the five-pixel plus rather than a single pixel.
pub const PLUS_SHARE: f64 = 0.13;
/// Star tones. Across the spectrum, not just blue-white: warm ambers and reds next to cyans is what
/// keeps a starfield from reading as grey noise. The hues are pulled most of the way to white and
/// pure white is weighted in four times over, so the field reads as a white starfield with colour
/// in it rather than as confetti.
pub const STAR_TONES: [&str; 14] = [
    "#ffffff", "#ffffff", "#ffffff", "#ffffff", "#fff8ec", "#ffecd0", "#ffcebd", "#ffdce4",
    "#f6f2ff", "#ece1ff", "#d4e2ff", "#bff1ff", "#c7fae9", "#fefecd",
];
/// How far the arms of a plus sit below its core.
pub const ARM_DIM: f64 = 0.50;
/// How far each quad shrinks inside its own pixel. A quad whose edges land on the lattice
/// rasterises as one pixel or two depending on which side of the fill rule the float falls, which
/// is what made the arms come out uneven; strictly containing the centre and nothing else is
/// decidable.
pub const STAR_INSET: f64 = 0.22;
/// Share of the stars that flicker at all.
pub const FLICKER_SHARE: f64 = 0.38;
/// How long one flicker level lasts, in milliseconds: stepped, not smooth, because it is pixel art.
pub const FLICKER_MS: f64 = 110.0;
/// The brightness levels a flickering star steps through, in order.
pub const FLICKER_LEVELS: [f64; 4] = [1.0, 0.82, 0.58, 0.82];

/// Segments per side of the vignette plane, so 15 × 15 vertices. Coarse on purpose: the falloff is
/// interpolated across these few quads and quantised by the render size.
const VIGNETTE_SEGMENTS: usize = 14;
/// Exponent of the vignette falloff: above one, so the sky stays open in the middle and darkens
/// quickly only towards the rim.
const VIGNETTE_FALLOFF: f64 = 1.6;
/// How far the falloff is squashed horizontally. The target is wider than it is tall, so the rim
/// has to arrive later across than down.
const VIGNETTE_SQUASH: f64 = 0.85;
/// Radius at which the falloff has reached the rim colour.
const VIGNETTE_REACH: f64 = 1.25;
/// Vertices per star cell: two triangles.
const CELL_VERTICES: usize = 6;

/// The backdrop: a vignette plane, the star quads, and the stars those quads belong to.
///
/// Both meshes are already in clip space — `x` and `y` in `-1..1`, `z` zero — for a pass with no
/// camera of its own, which is what the prototype's dedicated orthographic camera amounts to.
/// Neither carries normals or UVs: both are drawn unlit from vertex colours alone.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    /// The vignette. Its geometry does not depend on the render size; only the stars do.
    pub vignette: MeshData,
    /// The star quads. Their `colors` are what [`step_stars`] rewrites.
    pub stars: MeshData,
    /// One entry per star that survived clipping, in build order.
    pub list: Vec<Star>,
}

/// One star: which vertices of [`Space::stars`] it owns, and how it flickers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    /// Its first vertex in the star mesh.
    pub start: usize,
    /// How many vertices it owns, six per cell. The core comes first.
    pub count: usize,
    /// How many pixels it covers: one, or up to five for a plus whose arms all fell on screen.
    pub cells: usize,
    /// Its tone, `0..=255` per channel, before any dimming.
    pub tone: [u8; 3],
    /// Whether it is one of the plus-shaped ones.
    pub plus: bool,
    /// Whether it flickers at all.
    pub flick: bool,
    /// Where in the cycle it starts, in turns.
    pub phase: f64,
    /// How fast it runs through the cycle, in cycles per second.
    pub rate: f64,
}

/// The vignette plane: `PlaneGeometry(2, 2, 14, 14)` with a colour per grid point, expanded from
/// the plane's index into the plain triangle list every other mesh here is.
fn build_vignette() -> MeshData {
    let core = hex_rgb(SKY_CORE);
    let rim = hex_rgb(SKY_RIM);
    let side = VIGNETTE_SEGMENTS + 1;
    let seg = 2.0 / VIGNETTE_SEGMENTS as f64;
    let mut grid = Vec::with_capacity(side * side * 3);
    for iy in 0..side {
        for ix in 0..side {
            grid.extend([
                (ix as f64 * seg - 1.0) as f32,
                -(iy as f64 * seg - 1.0) as f32,
                0.0,
            ]);
        }
    }
    // The prototype reads the positions back out of the buffer, so the falloff is a function of the
    // `f32` grid point, not of the `f64` that produced it.
    let mut tint = Vec::with_capacity(side * side * 3);
    for v in 0..side * side {
        let x = f64::from(grid[v * 3]) * VIGNETTE_SQUASH;
        let y = f64::from(grid[v * 3 + 1]);
        let t = pow((hypot2(x, y) / VIGNETTE_REACH).min(1.0), VIGNETTE_FALLOFF);
        for k in 0..3 {
            let (a, b) = (f64::from(core[k]), f64::from(rim[k]));
            tint.push(((a + (b - a) * t) / 255.0) as f32);
        }
    }
    let mut mesh = MeshData::default();
    for iy in 0..VIGNETTE_SEGMENTS {
        for ix in 0..VIGNETTE_SEGMENTS {
            let a = ix + side * iy;
            let b = ix + side * (iy + 1);
            let c = ix + 1 + side * (iy + 1);
            let d = ix + 1 + side * iy;
            for v in [a, b, d, b, c, d] {
                mesh.positions.extend_from_slice(&grid[v * 3..v * 3 + 3]);
                mesh.colors.extend_from_slice(&tint[v * 3..v * 3 + 3]);
            }
        }
    }
    mesh
}

/// Appends the quad covering render pixel `(i, j)` in colour `c`, and answers how many cells that
/// added: none, when the pixel is off screen and the quad would have been clipped.
fn cell(mesh: &mut MeshData, w: i32, h: i32, i: i32, j: i32, c: [u8; 3]) -> usize {
    if i < 0 || j < 0 || i >= w || j >= h {
        return 0;
    }
    let (px, py) = (2.0 / f64::from(w), 2.0 / f64::from(h));
    let x0 = (f64::from(i) + STAR_INSET) * px - 1.0;
    let x1 = (f64::from(i) + 1.0 - STAR_INSET) * px - 1.0;
    let y1 = 1.0 - (f64::from(j) + STAR_INSET) * py;
    let y0 = 1.0 - (f64::from(j) + 1.0 - STAR_INSET) * py;
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y0), (x1, y1), (x0, y1)] {
        mesh.positions.extend([x as f32, y as f32, 0.0]);
        mesh.colors.extend([
            (f64::from(c[0]) / 255.0) as f32,
            (f64::from(c[1]) / 255.0) as f32,
            (f64::from(c[2]) / 255.0) as f32,
        ]);
    }
    1
}

/// The star field for a `w` × `h` render target: a fixed number of stars per pixel, each snapped to
/// one pixel of the lattice, a few of them plus-shaped. A star whose core falls off screen is
/// dropped whole.
fn build_stars(w: u32, h: u32) -> (MeshData, Vec<Star>) {
    let (wf, hf) = (f64::from(w), f64::from(h));
    let (wi, hi) = (w as i32, h as i32);
    let tones = STAR_TONES.map(hex_rgb);
    let mut mesh = MeshData::default();
    let mut list = Vec::new();
    let count = round(wf * hf * STAR_DENSITY).max(12.0) as i32;
    for s in 0..count {
        let i = (hash3i(s, 17, wi) * wf).floor() as i32;
        let j = (hash3i(s, 29, hi) * hf).floor() as i32;
        let tone = tones[(hash3i(s, 41, 7) * tones.len() as f64 * 0.999).floor() as usize];
        let plus = hash3i(s, 53, 11) < PLUS_SHARE;
        let start = mesh.vertex_count();
        if cell(&mut mesh, wi, hi, i, j, tone) == 0 {
            continue; // core clipped: drop the whole star
        }
        let mut cells = 1;
        if plus {
            let arm = tone.map(|v| round(f64::from(v) * ARM_DIM) as u8);
            cells += cell(&mut mesh, wi, hi, i - 1, j, arm)
                + cell(&mut mesh, wi, hi, i + 1, j, arm)
                + cell(&mut mesh, wi, hi, i, j - 1, arm)
                + cell(&mut mesh, wi, hi, i, j + 1, arm);
        }
        list.push(Star {
            start,
            count: mesh.vertex_count() - start,
            cells,
            tone,
            plus,
            flick: hash3i(s, 67, 13) < FLICKER_SHARE,
            phase: hash3i(s, 71, 19),
            // 0.6 to 1.7 cycles a second: fast enough to catch the eye, spread widely enough that
            // no two neighbours blink together.
            rate: 0.6 + hash3i(s, 83, 23) * 1.1,
        });
    }
    (mesh, list)
}

/// The backdrop for a `w` × `h` render target. The stars are pinned to the render-pixel lattice, so
/// every resize needs a fresh one; the vignette is the same whatever the size.
pub fn build_space(w: u32, h: u32) -> Space {
    let (stars, list) = build_stars(w, h);
    Space {
        vignette: build_vignette(),
        stars,
        list,
    }
}

/// Rewrites the colours of the flickering stars for the moment `t`, in milliseconds from any fixed
/// origin. `colors` is [`Space::stars`]'s colour array and `list` is [`Space::list`].
///
/// Only the stars that flicker are touched, and each steps between [`FLICKER_LEVELS`] rather than
/// fading: smooth twinkling would fight the pixel grid. How often to call this is the caller's
/// business — the prototype does so every [`FLICKER_MS`].
pub fn step_stars(colors: &mut [f32], list: &[Star], t: f64) {
    let levels = FLICKER_LEVELS.len();
    for s in list {
        if !s.flick {
            continue;
        }
        let phase = (t * 0.001 * s.rate + s.phase) % 1.0;
        let level = FLICKER_LEVELS[(phase * levels as f64).floor() as usize % levels];
        // `v` counts vertices, and a cell is six of them. Dimming on `v == 0` alone left five of
        // the core's own vertices at arm brightness, so the centre came out mottled and the plus
        // read as a fat blob instead of a cross.
        for v in 0..s.count {
            let dim = if v / CELL_VERTICES == 0 { 1.0 } else { ARM_DIM };
            for k in 0..3 {
                colors[(s.start + v) * 3 + k] = (f64::from(s.tone[k]) / 255.0 * level * dim) as f32;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn both_meshes_are_valid_triangle_lists() {
        let space = build_space(320, 180);
        assert!(space.vignette.validate().is_ok());
        assert!(space.stars.validate().is_ok());
        // 14 × 14 quads, two triangles each.
        assert_eq!(space.vignette.triangle_count(), 14 * 14 * 2);
        let cells: usize = space.list.iter().map(|s| s.cells).sum();
        assert_eq!(space.stars.vertex_count(), cells * CELL_VERTICES);
        assert_eq!(
            space.stars.vertex_count(),
            space.list.iter().map(|s| s.count).sum::<usize>()
        );
    }

    #[test]
    fn the_vignette_is_the_same_at_every_size() {
        assert_eq!(
            build_space(320, 180).vignette,
            build_space(300, 200).vignette
        );
    }

    #[test]
    fn a_star_never_leaves_its_own_pixel() {
        let (w, h) = (320u32, 180u32);
        let space = build_space(w, h);
        for v in 0..space.stars.vertex_count() {
            let x = f64::from(space.stars.positions[v * 3]);
            let y = f64::from(space.stars.positions[v * 3 + 1]);
            // The inset keeps every corner strictly inside the pixel it sits in, so the pixel a
            // corner lands in is the pixel its own centre lands in.
            let i = ((x + 1.0) / 2.0 * f64::from(w)).floor();
            let j = ((1.0 - y) / 2.0 * f64::from(h)).floor();
            assert!((0.0..f64::from(w)).contains(&i), "x {x} off screen");
            assert!((0.0..f64::from(h)).contains(&j), "y {y} off screen");
        }
    }

    #[test]
    fn flicker_steps_and_leaves_the_steady_stars_alone() {
        let space = build_space(320, 180);
        let mut colors = space.stars.colors.clone();
        step_stars(&mut colors, &space.list, 0.0);
        for s in &space.list {
            let head = s.start * 3;
            if s.flick {
                // A star at level 1.0 is exactly as bright as it was built.
                let phase = (s.phase * 4.0).floor() as usize;
                if FLICKER_LEVELS[phase] == 1.0 {
                    assert_eq!(colors[head], space.stars.colors[head]);
                } else {
                    assert!(colors[head] < space.stars.colors[head], "star not dimmed");
                }
            } else {
                assert_eq!(colors[head], space.stars.colors[head], "steady star moved");
            }
        }
        // Arms sit below their core whatever the level.
        let plus = space
            .list
            .iter()
            .find(|s| s.plus && s.flick && s.count > CELL_VERTICES)
            .unwrap();
        let core = colors[plus.start * 3];
        let arm = colors[(plus.start + CELL_VERTICES) * 3];
        assert_eq!(f64::from(arm), f64::from(core) * ARM_DIM);
    }
}
