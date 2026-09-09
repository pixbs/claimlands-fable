//! Meshes, textures and the two material pipelines, prototype section 5 (`docs/design/visuals.md`).
//!
//! One shader carries both materials: `fs_lambert` is three.js' `MeshLambertMaterial` with flat
//! normals, `fs_unlit` its `MeshBasicMaterial`. Everything that varies per mesh — the map, the
//! vertex colours, the alphaTest threshold, the see-through hole — is a uniform, so the pipeline
//! set stays small: one per distinct combination of cull, blend, depth, bias and topology.

use cl_model::{Filter, MeshData, Texture, Wrap};
use wgpu::util::DeviceExt;

/// Depth format of the low-resolution target. `Depth24Plus` is renderable everywhere the game runs,
/// WebGL2 included.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// One vertex: position, flat facet normal, atlas uv and vertex colour. Attributes the prototype's
/// geometry omits are filled with neutral values, so one layout serves every mesh.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// Model-space position.
    pub position: [f32; 3],
    /// Flat normal, repeated across the triangle.
    pub normal: [f32; 3],
    /// Texture coordinate before `uv_repeat` and `uv_offset`.
    pub uv: [f32; 2],
    /// Vertex colour, white where the mesh carries none.
    pub color: [f32; 3],
}

const VERTEX_ATTRS: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x3];

impl Vertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRS,
    };
}

/// Interleaves a [`MeshData`] into the one vertex layout, filling in what it does not carry.
pub fn vertices(data: &MeshData) -> Vec<Vertex> {
    let count = data.vertex_count();
    (0..count)
        .map(|i| Vertex {
            position: [
                data.positions[i * 3],
                data.positions[i * 3 + 1],
                data.positions[i * 3 + 2],
            ],
            normal: if data.normals.is_empty() {
                [0.0, 0.0, 1.0]
            } else {
                [
                    data.normals[i * 3],
                    data.normals[i * 3 + 1],
                    data.normals[i * 3 + 2],
                ]
            },
            uv: if data.uvs.is_empty() {
                [0.0, 0.0]
            } else {
                [data.uvs[i * 2], data.uvs[i * 2 + 1]]
            },
            color: if data.colors.is_empty() {
                [1.0, 1.0, 1.0]
            } else {
                [
                    data.colors[i * 3],
                    data.colors[i * 3 + 1],
                    data.colors[i * 3 + 2],
                ]
            },
        })
        .collect()
}

/// A mesh on the GPU.
#[derive(Debug)]
pub struct Mesh {
    buffer: wgpu::Buffer,
    count: u32,
}

impl Mesh {
    /// Vertices in the buffer.
    pub fn vertex_count(&self) -> u32 {
        self.count
    }

    /// `true` when there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// A texture on the GPU with the sampler its [`Texture`] asked for.
#[derive(Debug)]
pub struct GpuTexture {
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

/// Which fragment stage a material runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Shading {
    /// `diffuse × (ambient + saturate(dot(n, l)) × sun)`.
    #[default]
    Lambert,
    /// The diffuse colour alone.
    Unlit,
}

/// Which faces survive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Cull {
    /// Back faces dropped, three.js' `FrontSide`.
    #[default]
    Back,
    /// Nothing dropped, three.js' `DoubleSide`.
    None,
}

/// How a fragment reaches the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Blend {
    /// Overwrites the colour, leaving the target's alpha alone.
    #[default]
    Opaque,
    /// Straight alpha blending, for the halo.
    Alpha,
}

/// How a material uses the depth buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Depth {
    /// Tests and writes: the planet's solids.
    #[default]
    ReadWrite,
    /// Tests but does not write: layers that must not occlude what follows.
    ReadOnly,
    /// Neither: the space pass, drawn before the depth clear.
    Off,
}

/// Primitive topology. The prototype's hover ring is a `LineLoop`, which has no equivalent here:
/// close it by repeating the first vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Topology {
    /// Triangles.
    #[default]
    Triangles,
    /// A connected line strip.
    LineStrip,
}

/// Everything about a material that has to be baked into a pipeline. The rest — map, colour,
/// alphaTest, hole — rides in the uniform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MaterialDesc {
    /// Lit or not.
    pub shading: Shading,
    /// Face culling.
    pub cull: Cull,
    /// Blending.
    pub blend: Blend,
    /// Depth test and write.
    pub depth: Depth,
    /// three.js `polygonOffsetUnits` and `polygonOffsetFactor`, both negative to pull a coplanar
    /// layer toward the camera. `(0, 0)` for everything but the territory outline.
    pub depth_bias: (i32, i32),
    /// Triangles or lines.
    pub topology: Topology,
}

impl MaterialDesc {
    /// The planet's solids: lit, back-face culled, depth tested and written.
    pub fn lambert() -> Self {
        Self::default()
    }

    /// A lit material drawn from both sides, like the surf and the cloud decks.
    pub fn lambert_double_sided() -> Self {
        Self {
            cull: Cull::None,
            ..Self::default()
        }
    }

    /// Flat colour from both sides: the debug edges and the atmosphere.
    pub fn unlit() -> Self {
        Self {
            shading: Shading::Unlit,
            cull: Cull::None,
            ..Self::default()
        }
    }

    /// The territory outline: unlit, pulled `-2 / -2` toward the camera so it never fights the
    /// ground it lies on.
    pub fn border() -> Self {
        Self {
            depth_bias: (-2, -2),
            ..Self::unlit()
        }
    }

    /// The halo: unlit, alpha blended, depth tested but never written.
    pub fn halo() -> Self {
        Self {
            blend: Blend::Alpha,
            depth: Depth::ReadOnly,
            ..Self::unlit()
        }
    }

    /// The space pass: unlit and depth-free, drawn before the depth clear.
    pub fn space() -> Self {
        Self {
            depth: Depth::Off,
            ..Self::unlit()
        }
    }
}

/// The uniform block every draw carries.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawUniform {
    /// Model matrix, column-major.
    pub model: [f32; 16],
    /// Material colour, `w` the opacity.
    pub color: [f32; 4],
    /// `texture.repeat` in `xy`, `texture.offset` in `zw`.
    pub uv_transform: [f32; 4],
    /// `alphaTest`, then the see-through hole: cosine of its outer angle, cosine of its inner
    /// angle, and how far it is open (1 shut).
    pub params: [f32; 4],
    /// Direction the hole opens toward; `w` unused.
    pub focus: [f32; 4],
    /// [`FLAG_TEXTURED`], [`FLAG_VERTEX_COLOR`], [`FLAG_CLOUD_HOLE`].
    pub flags: u32,
    /// Padding to the 16-byte stride the uniform layout needs.
    pub pad: [u32; 3],
}

/// The material samples its map.
pub const FLAG_TEXTURED: u32 = 1;
/// The material multiplies in the mesh's vertex colours.
pub const FLAG_VERTEX_COLOR: u32 = 2;
/// The material opens the see-through hole toward [`DrawUniform::focus`].
pub const FLAG_CLOUD_HOLE: u32 = 4;

impl Default for DrawUniform {
    fn default() -> Self {
        Self {
            model: IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            uv_transform: [1.0, 1.0, 0.0, 0.0],
            params: [0.0, 0.0, 0.0, 1.0],
            focus: [0.0, 0.0, 1.0, 0.0],
            flags: 0,
            pad: [0; 3],
        }
    }
}

/// The column-major identity, the model matrix of anything not moved.
pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

/// The lights and camera shared by every draw of a pass.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneUniform {
    /// `projection * view`, column-major.
    pub view_proj: [f32; 16],
    /// Ambient colour already multiplied by its intensity; `w` unused.
    pub ambient: [f32; 4],
    /// Sun colour already multiplied by its intensity; `w` unused.
    pub sun_color: [f32; 4],
    /// Unit direction toward the sun in world space; `w` unused.
    pub sun_dir: [f32; 4],
}

impl Default for SceneUniform {
    fn default() -> Self {
        Self {
            view_proj: IDENTITY,
            ambient: light(AMBIENT_COLOR, AMBIENT_INTENSITY),
            sun_color: light(SUN_COLOR, SUN_INTENSITY),
            sun_dir: unit4(SUN_POSITION),
        }
    }
}

/// The prototype's ambient light colour.
pub const AMBIENT_COLOR: [u8; 3] = [0xb9, 0xc6, 0xff];
/// Its intensity.
pub const AMBIENT_INTENSITY: f32 = 0.66;
/// The prototype's directional light colour.
pub const SUN_COLOR: [u8; 3] = [0xff, 0xf2, 0xd8];
/// Its intensity.
pub const SUN_INTENSITY: f32 = 0.52;
/// Where the directional light sits; it aims at the origin, so the direction is this normalised.
pub const SUN_POSITION: [f32; 3] = [1.1, 0.9, 1.4];

/// A light colour times its intensity, as the shader wants it.
pub fn light(color: [u8; 3], intensity: f32) -> [f32; 4] {
    [
        f32::from(color[0]) / 255.0 * intensity,
        f32::from(color[1]) / 255.0 * intensity,
        f32::from(color[2]) / 255.0 * intensity,
        0.0,
    ]
}

fn unit4(v: [f32; 3]) -> [f32; 4] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let l = if l == 0.0 { 1.0 } else { l };
    [v[0] / l, v[1] / l, v[2] / l, 0.0]
}

/// A material: its uniform buffer, its bind group and the pipeline it draws with.
#[derive(Debug)]
pub struct Material {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: usize,
    /// What the material is drawn after: the prototype's `renderOrder`. The caller sorts by it.
    pub render_order: i32,
}

impl Material {
    /// Rewrites the uniform block, for the surf's stepping offset or the cloud hole.
    pub fn set(&self, queue: &wgpu::Queue, uniform: &DrawUniform) {
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(uniform));
    }
}

/// Pipelines, layouts and the scene uniform: everything shared by the draws of one target format.
#[derive(Debug)]
pub struct Renderer {
    format: wgpu::TextureFormat,
    shader: wgpu::ShaderModule,
    draw_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    pipelines: Vec<(MaterialDesc, wgpu::RenderPipeline)>,
    white: GpuTexture,
}

impl Renderer {
    /// Builds the shared state for a target of `format` with a [`DEPTH_FORMAT`] depth buffer.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("draw"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&scene_layout), Some(&draw_layout)],
            immediate_size: 0,
        });
        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene uniform"),
            contents: bytemuck::bytes_of(&SceneUniform::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });
        let white = white_texture(device, queue);
        Self {
            format,
            shader,
            draw_layout,
            pipeline_layout,
            scene_buffer,
            scene_bind_group,
            pipelines: Vec::new(),
            white,
        }
    }

    /// Rewrites the camera and lights for the next pass.
    pub fn set_scene(&self, queue: &wgpu::Queue, scene: &SceneUniform) {
        queue.write_buffer(&self.scene_buffer, 0, bytemuck::bytes_of(scene));
    }

    /// Uploads a mesh. Empty meshes are legal and draw nothing.
    pub fn upload_mesh(&self, device: &wgpu::Device, data: &MeshData) -> Mesh {
        let verts = vertices(data);
        Mesh {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            count: verts.len() as u32,
        }
    }

    /// Uploads a texture with the prototype's wrap and filter settings and no mipmaps.
    pub fn upload_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &Texture,
    ) -> GpuTexture {
        let size = wgpu::Extent3d {
            width: texture.image.width,
            height: texture.image.height,
            depth_or_array_layers: 1,
        };
        let gpu = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            gpu.as_image_copy(),
            &texture.image.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.image.width * 4),
                rows_per_image: Some(texture.image.height),
            },
            size,
        );
        let mode = |w: Wrap| match w {
            Wrap::Clamp => wgpu::AddressMode::ClampToEdge,
            Wrap::Repeat => wgpu::AddressMode::Repeat,
        };
        let filter = match texture.filter {
            Filter::Nearest => wgpu::FilterMode::Nearest,
            Filter::Linear => wgpu::FilterMode::Linear,
        };
        GpuTexture {
            view: gpu.create_view(&wgpu::TextureViewDescriptor::default()),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("texture"),
                address_mode_u: mode(texture.wrap_s),
                address_mode_v: mode(texture.wrap_t),
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            }),
        }
    }

    /// A material bound to `map`, or to a white texel when it has none.
    pub fn material(
        &mut self,
        device: &wgpu::Device,
        desc: MaterialDesc,
        uniform: &DrawUniform,
        map: Option<&GpuTexture>,
    ) -> Material {
        let pipeline = self.pipeline_index(device, desc);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("draw uniform"),
            contents: bytemuck::bytes_of(uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let map = map.unwrap_or(&self.white);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("draw"),
            layout: &self.draw_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&map.sampler),
                },
            ],
        });
        Material {
            uniform: buffer,
            bind_group,
            pipeline,
            render_order: 0,
        }
    }

    /// Records one draw. The caller has already begun a pass on a target of the renderer's format.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, material: &Material, mesh: &Mesh) {
        if mesh.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipelines[material.pipeline].1);
        pass.set_bind_group(0, &self.scene_bind_group, &[]);
        pass.set_bind_group(1, &material.bind_group, &[]);
        pass.set_vertex_buffer(0, mesh.buffer.slice(..));
        pass.draw(0..mesh.count, 0..1);
    }

    /// How many distinct pipelines have been built so far.
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    fn pipeline_index(&mut self, device: &wgpu::Device, desc: MaterialDesc) -> usize {
        if let Some(i) = self.pipelines.iter().position(|(d, _)| *d == desc) {
            return i;
        }
        self.pipelines.push((desc, self.build(device, desc)));
        self.pipelines.len() - 1
    }

    fn build(&self, device: &wgpu::Device, desc: MaterialDesc) -> wgpu::RenderPipeline {
        let blend = match desc.blend {
            Blend::Opaque => None,
            Blend::Alpha => Some(wgpu::BlendState::ALPHA_BLENDING),
        };
        // Opaque materials leave the target's alpha at the clear value, so the blit never reads a
        // hole punched by an alphaTest discard.
        let write_mask = match desc.blend {
            Blend::Opaque => wgpu::ColorWrites::COLOR,
            Blend::Alpha => wgpu::ColorWrites::ALL,
        };
        let (depth_write, depth_compare) = match desc.depth {
            Depth::ReadWrite => (true, wgpu::CompareFunction::Less),
            Depth::ReadOnly => (false, wgpu::CompareFunction::Less),
            Depth::Off => (false, wgpu::CompareFunction::Always),
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::LAYOUT)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some(match desc.shading {
                    Shading::Lambert => "fs_lambert",
                    Shading::Unlit => "fs_unlit",
                }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend,
                    write_mask,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: match desc.topology {
                    Topology::Triangles => wgpu::PrimitiveTopology::TriangleList,
                    Topology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
                },
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: match desc.cull {
                    Cull::Back => Some(wgpu::Face::Back),
                    Cull::None => None,
                },
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(depth_compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: desc.depth_bias.0,
                    slope_scale: desc.depth_bias.1 as f32,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }
}

/// One opaque white texel, bound wherever a material has no map so that the layout stays uniform.
/// White because the shader multiplies the material colour by it.
fn white_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("white"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &[255, 255, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    GpuTexture {
        view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
        sampler: device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("white"),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_pack_to_the_layout_the_shader_declares() {
        // std140: every member of a uniform block starts on a 16-byte boundary.
        assert_eq!(size_of::<SceneUniform>(), 64 + 3 * 16);
        assert_eq!(size_of::<DrawUniform>(), 64 + 4 * 16 + 16);
        assert_eq!(size_of::<DrawUniform>() % 16, 0);
        assert_eq!(size_of::<SceneUniform>() % 16, 0);
        assert_eq!(size_of::<Vertex>(), 11 * 4);
    }

    #[test]
    fn lights_match_the_prototype() {
        let scene = SceneUniform::default();
        // #b9c6ff x 0.66 and #fff2d8 x 0.52, straight, with no colour-space conversion.
        assert_eq!(scene.ambient[0], 0xb9 as f32 / 255.0 * 0.66);
        assert_eq!(scene.sun_color[2], 0xd8 as f32 / 255.0 * 0.52);
        // normalize(1.1, 0.9, 1.4)
        let l = (1.1f32 * 1.1 + 0.9 * 0.9 + 1.4 * 1.4).sqrt();
        assert_eq!(scene.sun_dir[0], 1.1 / l);
        assert_eq!(scene.sun_dir[1], 0.9 / l);
        assert_eq!(scene.sun_dir[2], 1.4 / l);
    }

    #[test]
    fn material_presets_match_the_visuals_table() {
        assert_eq!(MaterialDesc::border().depth_bias, (-2, -2));
        assert_eq!(MaterialDesc::border().cull, Cull::None);
        assert_eq!(MaterialDesc::halo().depth, Depth::ReadOnly);
        assert_eq!(MaterialDesc::halo().blend, Blend::Alpha);
        assert_eq!(MaterialDesc::space().depth, Depth::Off);
        assert_eq!(MaterialDesc::lambert_double_sided().cull, Cull::None);
        assert_eq!(MaterialDesc::lambert().shading, Shading::Lambert);
    }

    #[test]
    fn missing_attributes_become_neutral_vertices() {
        let data = MeshData {
            positions: vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            ..MeshData::default()
        };
        let v = vertices(&data);
        assert_eq!(v.len(), 3);
        assert_eq!(v[1].position, [3.0, 4.0, 5.0]);
        assert_eq!(
            v[1].color,
            [1.0, 1.0, 1.0],
            "white where there are no colours"
        );
        assert_eq!(v[1].uv, [0.0, 0.0]);
    }
}
