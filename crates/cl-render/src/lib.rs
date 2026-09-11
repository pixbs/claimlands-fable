//! wgpu renderer for Claim Lands: a pixel-scaled offscreen target blitted to the swapchain with
//! nearest sampling, exactly as the prototype renders at `1/pixelScale` and upscales the canvas.
#![forbid(unsafe_code)]

mod scene;

use cl_noise::js::round;
use wgpu::util::DeviceExt;

pub use scene::{
    AMBIENT_COLOR, AMBIENT_INTENSITY, Blend, Cull, DEPTH_FORMAT, Depth, DrawUniform,
    FLAG_CLOUD_HOLE, FLAG_SCREEN, FLAG_TEXTURED, FLAG_VERTEX_COLOR, GpuTexture, IDENTITY, Material,
    MaterialDesc, Mesh, Renderer, SUN_COLOR, SUN_INTENSITY, SUN_POSITION, SceneUniform, Shading,
    Topology, Vertex, light, vertices,
};
pub use wgpu::SurfaceTarget;

/// Why the renderer could not start.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// No GPU adapter accepted the surface.
    #[error("no compatible GPU adapter: {0}")]
    Adapter(String),
    /// The adapter refused the requested device.
    #[error("device request failed: {0}")]
    Device(String),
    /// The window or canvas could not become a surface.
    #[error("surface creation failed: {0}")]
    Surface(String),
}

/// Size of the low-resolution target for a window of `width` × `height` logical pixels at
/// `pixel_scale`: the prototype's `Math.max(1, Math.round(w / pixelScale))`.
pub fn target_size(width: u32, height: u32, pixel_scale: u32) -> (u32, u32) {
    let scale = f64::from(pixel_scale.max(1));
    let side = |v: u32| (round(f64::from(v) / scale) as u32).max(1);
    (side(width), side(height))
}

/// Logical (CSS) size of a surface of `width` × `height` physical pixels at `scale_factor`: the
/// prototype ignores the device pixel ratio (`setPixelRatio(1)`), so one render texel covers
/// `pixel_scale` CSS pixels whatever the screen density.
pub fn logical_size(width: u32, height: u32, scale_factor: f64) -> (u32, u32) {
    let s = if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    let side = |v: u32| (round(f64::from(v) / s) as u32).max(1);
    (side(width), side(height))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlitParams {
    to_linear: u32,
    _pad: [u32; 3],
}

/// Device, surface and the pixel-scaling pipeline.
pub struct Gpu {
    /// The device.
    pub device: wgpu::Device,
    /// Its queue.
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    scale_factor: f64,
    pixel_scale: u32,
    target: wgpu::Texture,
    target_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    blit_pipeline: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    blit_params: wgpu::Buffer,
    blit_bind_group: wgpu::BindGroup,
}

impl Gpu {
    /// Creates the renderer for a surface target of `width` × `height` physical pixels at the
    /// window's `scale_factor` (device pixel ratio).
    pub async fn new(
        target: impl Into<SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        scale_factor: f64,
        pixel_scale: u32,
    ) -> Result<Gpu, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(target)
            .map_err(|e| RenderError::Surface(e.to_string()))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| RenderError::Adapter(e.to_string()))?;
        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::default().using_resolution(adapter.limits())
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("claimlands"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .map_err(|e| RenderError::Device(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or_else(|| RenderError::Surface("surface not supported by the adapter".into()))?;
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            config.present_mode = wgpu::PresentMode::Fifo;
        }
        let format = config.format;
        surface.configure(&device, &config);
        log::info!(
            "surface {}x{} {:?} (srgb: {})",
            config.width,
            config.height,
            format,
            format.is_srgb()
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit"),
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let blit_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blit params"),
            contents: bytemuck::bytes_of(&BlitParams {
                to_linear: u32::from(format.is_srgb()),
                _pad: [0; 3],
            }),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let (lw, lh) = logical_size(config.width, config.height, scale_factor);
        let (tw, th) = target_size(lw, lh, pixel_scale);
        let (target, target_view) = make_target(&device, tw, th);
        let depth_view = make_depth(&device, tw, th);
        let blit_bind_group = make_bind_group(
            &device,
            &blit_layout,
            &target_view,
            &blit_sampler,
            &blit_params,
        );
        Ok(Gpu {
            device,
            queue,
            surface,
            config,
            scale_factor,
            pixel_scale: pixel_scale.max(1),
            target,
            target_view,
            depth_view,
            blit_pipeline,
            blit_layout,
            blit_sampler,
            blit_params,
            blit_bind_group,
        })
    }

    /// Follows a window resize (physical pixels) or a density change.
    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.scale_factor = scale_factor;
        self.surface.configure(&self.device, &self.config);
        self.rebuild_target();
    }

    /// Changes how many screen pixels one render texel covers.
    pub fn set_pixel_scale(&mut self, pixel_scale: u32) {
        self.pixel_scale = pixel_scale.max(1);
        self.rebuild_target();
    }

    /// Current pixel scale.
    pub fn pixel_scale(&self) -> u32 {
        self.pixel_scale
    }

    /// Swapchain size in physical pixels.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Size of the low-resolution target in texels.
    pub fn target_extent(&self) -> (u32, u32) {
        (self.target.width(), self.target.height())
    }

    /// Format of the low-resolution target: what a [`Renderer`] must be built for.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target.format()
    }

    /// Swapchain format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    fn rebuild_target(&mut self) {
        let (lw, lh) = logical_size(self.config.width, self.config.height, self.scale_factor);
        let (tw, th) = target_size(lw, lh, self.pixel_scale);
        if (tw, th) == self.target_extent() {
            return;
        }
        let (target, view) = make_target(&self.device, tw, th);
        self.target = target;
        self.target_view = view;
        self.depth_view = make_depth(&self.device, tw, th);
        self.blit_bind_group = make_bind_group(
            &self.device,
            &self.blit_layout,
            &self.target_view,
            &self.blit_sampler,
            &self.blit_params,
        );
    }

    /// Acquires the next swapchain image. `None` means the frame is skipped (timeout, occluded,
    /// or the surface had to be reconfigured and will be ready next time).
    pub fn frame(&self) -> Option<Frame<'_>> {
        use wgpu::CurrentSurfaceTexture as Cur;
        let surface_texture = match self.surface.get_current_texture() {
            Cur::Success(t) | Cur::Suboptimal(t) => t,
            Cur::Outdated | Cur::Lost => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    Cur::Success(t) | Cur::Suboptimal(t) => t,
                    other => {
                        log::warn!("surface unavailable after reconfigure: {other:?}");
                        return None;
                    }
                }
            }
            other => {
                log::debug!("frame skipped: {other:?}");
                return None;
            }
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        Some(Frame {
            gpu: self,
            surface_texture: Some(surface_texture),
            surface_view,
            encoder: Some(encoder),
        })
    }
}

/// The depth buffer of the low-resolution target. The prototype clears depth between its two
/// passes rather than keeping two buffers.
fn make_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn make_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("low-res target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    (target, view)
}

fn make_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

/// One frame in flight: draw into the low-resolution target, then `finish` blits and presents.
pub struct Frame<'a> {
    gpu: &'a Gpu,
    surface_texture: Option<wgpu::SurfaceTexture>,
    surface_view: wgpu::TextureView,
    encoder: Option<wgpu::CommandEncoder>,
}

impl Frame<'_> {
    /// The full-resolution swapchain view, for layers drawn after the blit (the UI).
    pub fn surface_view(&self) -> &wgpu::TextureView {
        &self.surface_view
    }

    /// The low-resolution target view.
    pub fn target_view(&self) -> &wgpu::TextureView {
        &self.gpu.target_view
    }

    /// The frame's command encoder.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .as_mut()
            .expect("encoder is present until finish")
    }

    /// Clears the low-resolution target to `clear` (raw prototype colour, 0–1 per channel).
    pub fn clear_target(&mut self, clear: [f64; 3]) {
        let view = self.gpu.target_view.clone();
        let encoder = self.encoder();
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear target"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear[0],
                        g: clear[1],
                        b: clear[2],
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    /// Begins a pass on the low-resolution target and its depth buffer.
    ///
    /// The prototype draws its two passes into one buffer: space first with depth off, then
    /// `renderer.clearDepth()`, then the planet. Pass `clear_color` on the first pass of a frame
    /// and `clear_depth` on the one that starts the planet.
    pub fn scene_pass(
        &mut self,
        clear_color: Option<[f64; 3]>,
        clear_depth: bool,
    ) -> wgpu::RenderPass<'static> {
        let view = self.gpu.target_view.clone();
        let depth = self.gpu.depth_view.clone();
        let encoder = self.encoder();
        encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match clear_color {
                            Some(c) => wgpu::LoadOp::Clear(wgpu::Color {
                                r: c[0],
                                g: c[1],
                                b: c[2],
                                a: 1.0,
                            }),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: if clear_depth {
                            wgpu::LoadOp::Clear(1.0)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime()
    }
    /// Blits the target onto the swapchain (nearest, whole screen).
    pub fn blit(&mut self) {
        let view = self.surface_view.clone();
        let gpu = self.gpu;
        let encoder = self.encoder();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&gpu.blit_pipeline);
        pass.set_bind_group(0, &gpu.blit_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Submits everything recorded so far and presents.
    pub fn finish(mut self) {
        let encoder = self.encoder.take().expect("encoder present");
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        if let Some(t) = self.surface_texture.take() {
            self.gpu.queue.present(t);
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn target_size_rounds_like_the_prototype() {
        assert_eq!(target_size(900, 600, 3), (300, 200));
        assert_eq!(target_size(901, 599, 3), (300, 200));
        assert_eq!(target_size(1, 1, 3), (1, 1));
        assert_eq!(target_size(1000, 500, 0), (1000, 500));
        assert_eq!(target_size(7, 7, 2), (4, 4), "3.5 rounds up");
        assert_eq!(logical_size(1800, 1200, 2.0), (900, 600));
        assert_eq!(
            logical_size(900, 600, 0.0),
            (900, 600),
            "a zero scale factor is treated as 1"
        );
    }
}
