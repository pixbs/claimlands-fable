//! The backdrop on the GPU: the vignette and the stars of [`cl_scenery::build_space`], drawn in
//! their own pass before the planet.
//!
//! The stars are snapped to the render-pixel lattice, so the whole thing belongs to one target size
//! and is rebuilt whenever that size changes — which is what the prototype's `resize` does. The
//! clock that steps their flicker arrives with the other animation clocks (#19).

use cl_render::{
    DrawUniform, FLAG_SCREEN, FLAG_VERTEX_COLOR, Material, MaterialDesc, Mesh, Renderer,
};
use cl_scenery::build_space;

/// The space pass: the two meshes of the backdrop and the one material both are drawn with.
pub struct Backdrop {
    /// Render target size the stars were snapped to.
    size: (u32, u32),
    vignette: Mesh,
    stars: Mesh,
    material: Material,
}

impl Backdrop {
    /// Builds and uploads the backdrop for a `w` × `h` render target.
    pub fn new(renderer: &mut Renderer, device: &wgpu::Device, w: u32, h: u32) -> Self {
        let space = build_space(w, h);
        Self {
            size: (w, h),
            vignette: renderer.upload_mesh(device, &space.vignette),
            stars: renderer.upload_mesh(device, &space.stars),
            material: renderer.material(
                device,
                MaterialDesc::space(),
                &DrawUniform {
                    flags: FLAG_SCREEN | FLAG_VERTEX_COLOR,
                    ..DrawUniform::default()
                },
                None,
            ),
        }
    }

    /// Re-snaps the stars to a new target size. A no-op while the size is unchanged, so it is safe
    /// to call every frame.
    pub fn resize(&mut self, renderer: &Renderer, device: &wgpu::Device, w: u32, h: u32) {
        if self.size == (w, h) {
            return;
        }
        self.size = (w, h);
        // Only the stars follow the lattice; the vignette is the same plane at every size.
        self.stars = renderer.upload_mesh(device, &build_space(w, h).stars);
    }

    /// Draws the vignette, then the stars over it.
    pub fn draw(&self, renderer: &Renderer, pass: &mut wgpu::RenderPass<'_>) {
        renderer.draw(pass, &self.material, &self.vignette);
        renderer.draw(pass, &self.material, &self.stars);
    }
}
