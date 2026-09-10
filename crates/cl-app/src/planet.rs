//! The planet on the GPU: the ported builders assembled once, then drawn every frame.
//!
//! What is here is what has been ported. The cover meshes (fields, forest, villages), the clouds,
//! the halo and the space pass are their own issues and join this list as they land; each is one
//! more mesh and one more material, not a change to how the scene is put together.

use cl_hexsphere::{Frames, HexSphere, compute_tile_frames};
use cl_model::{Texture, WorldSnapshot, hex_rgb};
use cl_pixelart::{
    Atlas, FOAM_FRAMES, build_terrain_atlas, make_cliff_texture, make_field_texture,
    make_foam_texture, palette,
};
use cl_render::{
    DrawUniform, FLAG_TEXTURED, FLAG_VERTEX_COLOR, GpuTexture, Material, MaterialDesc, Mesh,
    Renderer,
};
use cl_scenery::{
    Fields, Forest, Houses, Terrain, build_atmosphere, build_fields, build_forest, build_houses,
    build_terrain,
};

/// The surf steps one frame every this many milliseconds.
pub const FOAM_MS: f64 = 140.0;
/// `alphaTest` of the surf: its sheet is either a foam texel or nothing.
pub const FOAM_ALPHA_TEST: f32 = 0.5;

/// One mesh, the material it is drawn with, and the uniform behind that material.
struct Part {
    mesh: Mesh,
    material: Material,
    uniform: DrawUniform,
}

impl Part {
    fn upload(
        renderer: &mut Renderer,
        device: &wgpu::Device,
        data: &cl_model::MeshData,
        desc: MaterialDesc,
        uniform: DrawUniform,
        map: Option<&GpuTexture>,
    ) -> Self {
        Self {
            mesh: renderer.upload_mesh(device, data),
            material: renderer.material(device, desc, &uniform, map),
            uniform,
        }
    }

    fn flush(&self, queue: &wgpu::Queue) {
        self.material.set(queue, &self.uniform);
    }
}

/// The world, its meshes, and the GPU resources they are drawn from.
pub struct Planet {
    /// The tiling.
    pub sphere: HexSphere,
    /// What stands on it.
    pub snapshot: WorldSnapshot,
    /// Per-tile frames.
    pub frames: Frames,
    /// The ground atlas.
    pub atlas: Atlas,
    /// The surface meshes.
    pub terrain: Terrain,
    /// The farmland, where any grows.
    pub fields: Option<Fields>,
    /// The woods, where any grow.
    pub forest: Option<Forest>,
    /// The villages, where any stand.
    pub houses: Option<Houses>,
    ground: Part,
    walls: Part,
    foam: Part,
    air: Part,
    /// Field tops and sides, the fence posts, the crowns, then the villages — the prototype's
    /// cover order. Empty where the world grew no cover at all.
    cover: Vec<Part>,
    foam_frame: u32,
    foam_at: f64,
}

/// A palette colour as the shader wants it: raw, with no colour-space conversion, exactly as
/// three.js passes a hex value through under `LinearEncoding`.
fn color(hex: &str) -> [f32; 4] {
    let c = hex_rgb(hex);
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        1.0,
    ]
}

/// `uv_transform` of the surf at frame `f`: the sheet stacks every frame, so `repeat.y` is one
/// frame's share of it and the offset walks up the stack.
fn foam_uv(frame: u32) -> [f32; 4] {
    let share = 1.0 / FOAM_FRAMES as f32;
    [1.0, share, 0.0, frame as f32 * share]
}

impl Planet {
    /// Builds every ported layer for `snapshot` and uploads it.
    pub fn new(
        renderer: &mut Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sphere: HexSphere,
        snapshot: WorldSnapshot,
    ) -> Self {
        let seed = f64::from(snapshot.seed);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let atlas = build_terrain_atlas(&sphere, &frames, &snapshot, seed);
        let terrain = build_terrain(&sphere, &frames, &snapshot, &atlas);
        let shell = build_atmosphere(&sphere, frames.px);
        let fields = build_fields(&sphere, &frames, &snapshot, frames.px);
        let forest = build_forest(&sphere, &frames, &snapshot, frames.px, seed);
        let houses = build_houses(&sphere, &frames, &snapshot, frames.px, seed);

        let upload = |t: &Texture| -> GpuTexture { renderer.upload_texture(device, queue, t) };
        let atlas_map = upload(&atlas.texture);
        let cliff_map = upload(&make_cliff_texture());
        let foam_map = upload(&make_foam_texture());
        let field_map = upload(&make_field_texture());

        let textured = DrawUniform {
            flags: FLAG_TEXTURED | FLAG_VERTEX_COLOR,
            ..DrawUniform::default()
        };
        let ground = Part::upload(
            renderer,
            device,
            &terrain.mesh,
            MaterialDesc::lambert(),
            textured,
            Some(&atlas_map),
        );
        let walls = Part::upload(
            renderer,
            device,
            &terrain.walls,
            MaterialDesc::lambert(),
            textured,
            Some(&cliff_map),
        );
        let foam = Part::upload(
            renderer,
            device,
            &terrain.foam,
            MaterialDesc::lambert_double_sided(),
            DrawUniform {
                uv_transform: foam_uv(0),
                params: [FOAM_ALPHA_TEST, 0.0, 0.0, 1.0],
                flags: FLAG_TEXTURED,
                ..DrawUniform::default()
            },
            Some(&foam_map),
        );
        let air = Part::upload(
            renderer,
            device,
            &shell.mesh,
            MaterialDesc::unlit(),
            DrawUniform {
                color: color(palette::AIR_COLOR),
                ..DrawUniform::default()
            },
            None,
        );

        // Farmland: the tops and sides sample the field strip, the posts are flat vertex colour.
        let mut cover = Vec::new();
        if let Some(f) = &fields {
            cover.push(Part::upload(
                renderer,
                device,
                &f.surface,
                MaterialDesc::lambert(),
                textured,
                Some(&field_map),
            ));
            if !f.posts.is_empty() {
                cover.push(Part::upload(
                    renderer,
                    device,
                    &f.posts,
                    MaterialDesc::lambert(),
                    DrawUniform {
                        flags: FLAG_VERTEX_COLOR,
                        ..DrawUniform::default()
                    },
                    None,
                ));
            }
        }
        // The wood: crowns and their floor discs carry their own flat colours, no texture.
        if let Some(w) = &forest {
            cover.push(Part::upload(
                renderer,
                device,
                &w.surface,
                MaterialDesc::lambert(),
                DrawUniform {
                    flags: FLAG_VERTEX_COLOR,
                    ..DrawUniform::default()
                },
                None,
            ));
        }
        // Villages: walls, roofs, openings and chimneys, flat vertex colour like the wood.
        if let Some(h) = &houses {
            cover.push(Part::upload(
                renderer,
                device,
                &h.surface,
                MaterialDesc::lambert(),
                DrawUniform {
                    flags: FLAG_VERTEX_COLOR,
                    ..DrawUniform::default()
                },
                None,
            ));
        }

        Self {
            sphere,
            snapshot,
            frames,
            atlas,
            terrain,
            fields,
            forest,
            houses,
            ground,
            walls,
            foam,
            air,
            cover,
            foam_frame: 0,
            foam_at: 0.0,
        }
    }

    /// Advances the surf. `now` is milliseconds from any fixed origin; returns `true` when the
    /// frame changed.
    pub fn animate(&mut self, queue: &wgpu::Queue, now: f64) -> bool {
        if now - self.foam_at <= FOAM_MS {
            return false;
        }
        self.foam_at = now;
        self.foam_frame = (self.foam_frame + 1) % FOAM_FRAMES;
        self.foam.uniform.uv_transform = foam_uv(self.foam_frame);
        self.foam.flush(queue);
        true
    }

    /// Points every layer at the planet's current orientation.
    pub fn set_model(&mut self, queue: &wgpu::Queue, model: [f32; 16]) {
        let fixed = [
            &mut self.ground,
            &mut self.walls,
            &mut self.foam,
            &mut self.air,
        ];
        for part in fixed.into_iter().chain(self.cover.iter_mut()) {
            part.uniform.model = model;
            part.flush(queue);
        }
    }

    /// Draws the planet: opaque solids first, then the surf, as the prototype's `renderOrder` asks.
    pub fn draw(&self, renderer: &Renderer, pass: &mut wgpu::RenderPass<'_>) {
        for part in [&self.ground, &self.walls]
            .into_iter()
            .chain(self.cover.iter())
            .chain([&self.air, &self.foam])
        {
            renderer.draw(pass, &part.material, &part.mesh);
        }
    }
}
