//! The halo on the GPU: one quad parked behind the planet, prototype section 5 and its per-frame
//! placement.
//!
//! It belongs to the scene rather than to the planet, so the trackball never touches it: dragging
//! spins the world underneath a halo that stays put, which is what makes the air read as air rather
//! than as a decal stuck to the globe. The camera only ever sits on `+z` looking at the origin, so
//! a plane in `xy` faces it squarely and needs no billboarding.

use cl_model::MeshData;
use cl_pixelart::{GLOW_BACK, GLOW_OUT, make_glow};
use cl_render::{DrawUniform, FLAG_TEXTURED, Material, MaterialDesc, Mesh, Renderer};

/// Where the halo quad sits this frame: how far back along `-z`, and the half-extent it is scaled
/// to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Distance behind the origin, in world units.
    pub back: f64,
    /// Half-width and half-height of the quad, in world units.
    pub scale: f64,
}

/// Where to park the halo for an atmosphere of `air_radius` seen from `camera_distance`.
///
/// The quad sits behind the planet, so perspective shrinks it. Scaling by the ratio of the two
/// distances — the camera's to the quad and the camera's to the origin — cancels that exactly, so
/// the halo keeps the same apparent size against the atmosphere at every zoom. Without the ratio it
/// would swell as the camera pulled back and shrink as it closed in, which reads as the air
/// detaching from the planet.
pub fn placement(air_radius: f64, camera_distance: f64) -> Placement {
    let back = air_radius * GLOW_BACK;
    Placement {
        back,
        scale: air_radius * GLOW_OUT * (camera_distance + back) / camera_distance,
    }
}

/// The quad: two triangles spanning `-1..1` in `xy` at `z = 0`, the prototype's `PlaneGeometry(2, 2)`
/// before the scale is applied. Unlit, so it carries no normals.
fn quad() -> MeshData {
    let mut mesh = MeshData::default();
    // Counter-clockwise seen from `+z`, though the material is double sided either way.
    let corners = [
        ([-1.0, -1.0], [0.0, 0.0]),
        ([1.0, -1.0], [1.0, 0.0]),
        ([1.0, 1.0], [1.0, 1.0]),
        ([-1.0, -1.0], [0.0, 0.0]),
        ([1.0, 1.0], [1.0, 1.0]),
        ([-1.0, 1.0], [0.0, 1.0]),
    ];
    for ([x, y], [u, v]) in corners {
        mesh.positions.extend([x as f32, y as f32, 0.0]);
        mesh.uvs.extend([u as f32, v as f32]);
    }
    mesh
}

/// The halo: its quad, its material, and the uniform that places it.
pub struct Halo {
    mesh: Mesh,
    material: Material,
    uniform: DrawUniform,
    air_radius: f64,
}

impl Halo {
    /// Uploads the gradient and its quad. `air_radius` is the atmosphere shell's radius, which is
    /// what the halo is sized against.
    pub fn new(
        renderer: &mut Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        air_radius: f64,
    ) -> Self {
        let map = renderer.upload_texture(device, queue, &make_glow());
        let uniform = DrawUniform {
            flags: FLAG_TEXTURED,
            ..DrawUniform::default()
        };
        Self {
            mesh: renderer.upload_mesh(device, &quad()),
            material: renderer.material(device, MaterialDesc::halo(), &uniform, Some(&map)),
            uniform,
            air_radius,
        }
    }

    /// Places the quad for the camera's distance. No rotation: the halo does not spin with the
    /// planet.
    pub fn set_camera_distance(&mut self, queue: &wgpu::Queue, distance: f32) {
        let p = placement(self.air_radius, f64::from(distance));
        let (s, back) = (p.scale as f32, p.back as f32);
        // Column-major, and only ever a scale in `xy` and a shove along `-z`.
        self.uniform.model = [
            s, 0.0, 0.0, 0.0, //
            0.0, s, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, -back, 1.0,
        ];
        self.material.set(queue, &self.uniform);
    }

    /// Draws the halo. The prototype gives it `renderOrder = -1`, so it goes down before the planet
    /// and, writing no depth, never occludes what follows.
    pub fn draw(&self, renderer: &Renderer, pass: &mut wgpu::RenderPass<'_>) {
        renderer.draw(pass, &self.material, &self.mesh);
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;
    use crate::camera::{DIST_MAX, DIST_MIN, DIST_START};

    /// The atmosphere radius of the default world, near enough for the ratios below.
    const AIR: f64 = 1.157;

    #[test]
    fn the_halo_stays_pinned_to_the_atmosphere_at_every_zoom() {
        // What has to hold is not that the halo's apparent size is fixed — it grows and shrinks
        // with the zoom like everything else — but that it stays the same multiple of the
        // atmosphere's. The quad is further from the camera than the planet, so perspective shrinks
        // it more; the `(d + back) / d` factor is what cancels that.
        for d in [
            DIST_MIN.into(),
            2.0,
            DIST_START.into(),
            4.5,
            DIST_MAX.into(),
        ] {
            let p = placement(AIR, d);
            // Apparent half-extent = size / distance to it, for the quad and for the shell.
            let halo = p.scale / (d + p.back);
            let air = AIR / d;
            assert!(
                (halo / air - GLOW_OUT).abs() < 1e-12,
                "at distance {d} the halo is {} atmospheres, not {GLOW_OUT}",
                halo / air
            );
        }
    }

    #[test]
    fn the_quad_clears_the_air_shell_in_depth() {
        // Parked strictly behind the whole shell, or the planet would punch through it.
        let p = placement(AIR, DIST_START.into());
        assert!(p.back > AIR, "{} is not behind {AIR}", p.back);
        assert_eq!(p.back, AIR * GLOW_BACK);
    }

    #[test]
    fn the_quad_covers_the_atmosphere() {
        // Closer in the quad has to grow, since it is further from the camera than the planet is.
        let near = placement(AIR, DIST_MIN.into());
        let far = placement(AIR, DIST_MAX.into());
        assert!(near.scale > far.scale, "{near:?} vs {far:?}");
        // Even zoomed all the way out it still reaches past the atmosphere it is drawn behind.
        assert!(far.scale > AIR, "{} does not cover {AIR}", far.scale);
    }

    #[test]
    fn the_quad_is_two_triangles_with_uvs() {
        let mesh = quad();
        assert_eq!(mesh.vertex_count(), 6);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.uvs.len(), mesh.vertex_count() * 2);
        assert!(mesh.validate().is_ok());
        // Flat at z = 0 and spanning the unit square, so the scale alone decides how big it lands.
        for (i, v) in mesh.positions.iter().enumerate() {
            if i % 3 == 2 {
                assert_eq!(*v, 0.0, "z at {i}");
            } else {
                assert_eq!(v.abs(), 1.0, "corner at {i}");
            }
        }
    }
}
