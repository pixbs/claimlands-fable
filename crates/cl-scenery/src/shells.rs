//! The two shells built from the tiling alone: the atmosphere rim and the cloud deck shell, plus
//! the stack of decks that ride the second one.

use cl_hexsphere::{Frames, HexSphere};
use cl_model::MeshData;
use cl_model::world::{ATMO_PX, CLOUD_LIFT_PX, CLOUD_PX, RADIUS};
use cl_noise::js::{cos, hypot2, sin};
use cl_noise::vec::{add, mul, norm};
use cl_pixelart::CLOUD_DECKS;

/// A mesh on a sphere shell, with the radius the app needs for the halo and the deck spacing.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    /// Shell radius in world units.
    pub radius: f64,
    /// The mesh.
    pub mesh: MeshData,
}

/// The atmosphere: the same tiling `ATMO_PX` above the land shell, built with its winding
/// reversed. The near half becomes back-facing and is culled, so it never hides the planet, while
/// the far half shows as a rim beyond the silhouette. Unlit and untextured: one flat tone.
pub fn build_atmosphere(sphere: &HexSphere, px: f64) -> Shell {
    let rad = RADIUS + ATMO_PX * px;
    let mut mesh = MeshData::default();
    for t in &sphere.tiles {
        let mut mid = [0.0, 0.0, 0.0];
        for &p in &t.corners {
            mid = add(mid, p);
        }
        let mid = mul(mid, rad / t.sides() as f64);
        let nn = mul(norm(mid), -1.0); // normals point inward, like the faces
        for k in 0..t.sides() {
            let a = mul(t.corners[k], rad);
            let b = mul(t.corners[(k + 1) % t.sides()], rad);
            // b before a: this is the inversion
            mesh.push_vertex(mid, nn);
            mesh.push_vertex(b, nn);
            mesh.push_vertex(a, nn);
        }
    }
    Shell { radius: rad, mesh }
}

/// One shell shared by every cloud deck, `ATMO_PX + CLOUD_PX` above the land, mapped
/// cylindrically and equal-area in latitude (`v` tracks the sine, not the angle) so a texture
/// offset equals a rotation of the sky and one texel covers one world pixel everywhere. Tiles on
/// the axis have no longitude and take a single flat sample.
pub fn build_cloud_shell(sphere: &HexSphere, frames: &Frames, px: f64) -> Shell {
    let rad = RADIUS + (ATMO_PX + CLOUD_PX) * px;
    let tau = 2.0 * std::f64::consts::PI;
    let lon = |n: [f64; 3]| libm::atan2(n[2], n[0]) / tau + 0.5;
    let lat = |n: [f64; 3]| 0.5 + 0.5 * n[1];
    let mut mesh = MeshData::default();
    for (t, frame) in sphere.tiles.iter().zip(&frames.tiles) {
        let mut mid = [0.0, 0.0, 0.0];
        for &p in &t.corners {
            mid = add(mid, p);
        }
        let nc = norm(mid);
        let mid = mul(nc, rad);
        let polar = hypot2(nc[0], nc[2]) < sin(frame.unit_circum * 1.05);
        let u0 = lon(nc);
        let u_of = |n: [f64; 3]| {
            if polar {
                return u0;
            }
            let mut u = lon(n);
            while u - u0 > 0.5 {
                u -= 1.0;
            }
            while u0 - u > 0.5 {
                u += 1.0;
            }
            u
        };
        let v_of = |n: [f64; 3]| if polar { lat(nc) } else { lat(n) };
        let uc = [u0, lat(nc)];
        for k in 0..t.sides() {
            let ca = t.corners[k];
            let cb = t.corners[(k + 1) % t.sides()];
            mesh.push_vertex(mid, nc);
            mesh.push_vertex(mul(ca, rad), nc);
            mesh.push_vertex(mul(cb, rad), nc);
            mesh.uvs.extend([
                uc[0] as f32,
                uc[1] as f32,
                u_of(ca) as f32,
                v_of(ca) as f32,
                u_of(cb) as f32,
                v_of(cb) as f32,
            ]);
        }
    }
    Shell { radius: rad, mesh }
}

/// Resting outer angle of the see-through hole, in radians. The camera's distance drives the cap
/// from here (its own issue); at rest the hole is shut and these only seed the uniform.
pub const HOLE_REST_OUT: f64 = 0.7;
/// Resting inner angle of the see-through hole, in radians.
pub const HOLE_REST_IN: f64 = 0.3;
/// How far the hole is open at rest: 1 is shut, so the cap has no effect until something opens it.
pub const HOLE_REST_OPEN: f64 = 1.0;

/// The resting cap as the deck's shader wants it: cosine of the outer angle, cosine of the inner,
/// and how far the hole is open. Shut at rest, so it costs the fragment nothing until opened.
pub fn hole_rest() -> [f64; 3] {
    [cos(HOLE_REST_OUT), cos(HOLE_REST_IN), HOLE_REST_OPEN]
}

/// Where one cloud deck sits on the shared shell. The decks differ only in how far they are scaled
/// out, the tint they are drawn in, and the order they are drawn in — the geometry is the same
/// shell for all three, as it is in the prototype.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deck {
    /// Factor the shared shell is scaled by, so deck `k` sits `k * CLOUD_LIFT_PX` above the first.
    pub scale: f64,
    /// Tint the deck is drawn in; the texture itself is white.
    pub tone: &'static str,
    /// The prototype's `renderOrder`: decks are drawn low to high.
    pub render_order: i32,
}

/// The cloud stack: one shell and the three decks that ride it, outermost tint last.
#[derive(Debug, Clone, PartialEq)]
pub struct Clouds {
    /// The shell every deck is drawn from.
    pub shell: Shell,
    /// The decks, parallel to `cl_pixelart::CLOUD_DECKS`.
    pub decks: [Deck; 3],
}

/// The cloud stack for a world. Scaling one shell rather than building three keeps the decks
/// exactly concentric and uploads a third of the geometry; the prototype shares its geometry the
/// same way.
pub fn build_clouds(sphere: &HexSphere, frames: &Frames, px: f64) -> Clouds {
    let shell = build_cloud_shell(sphere, frames, px);
    let decks = std::array::from_fn(|k| Deck {
        scale: (shell.radius + k as f64 * CLOUD_LIFT_PX * px) / shell.radius,
        tone: CLOUD_DECKS[k].tone,
        render_order: k as i32,
    });
    Clouds { shell, decks }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_hexsphere::compute_tile_frames;

    use super::*;

    #[test]
    fn shells_are_valid_and_sized() {
        let sphere = HexSphere::build(2);
        let frames = compute_tile_frames(&sphere, &vec![0; sphere.len()]);
        let air = build_atmosphere(&sphere, frames.px);
        assert!(air.mesh.validate().is_ok());
        assert_eq!(air.mesh.vertex_count(), 42 * 6 * 3 - 12 * 3);
        let clouds = build_cloud_shell(&sphere, &frames, frames.px);
        assert!(clouds.mesh.validate().is_ok());
        assert!(clouds.radius > air.radius);
        assert_eq!(clouds.mesh.uvs.len(), clouds.mesh.vertex_count() * 2);
    }
}
