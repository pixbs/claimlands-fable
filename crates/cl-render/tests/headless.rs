#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Renders one frame without a window and reads the texels back.
//!
//! This is the only test that proves the pipelines compile and that the Lambert formula lands the
//! prototype's numbers on a real target. It needs an adapter: a GPU, or lavapipe on CI. Where none
//! is available it prints why and passes, so a laptop without a working driver does not fail the
//! suite — CI installs lavapipe and does run it.

use cl_model::{Filter, MeshData, RgbaImage, Texture, Wrap};
use cl_render::{
    DEPTH_FORMAT, DrawUniform, FLAG_SCREEN, FLAG_TEXTURED, FLAG_VERTEX_COLOR, IDENTITY, Material,
    MaterialDesc, Mesh, Renderer, SceneUniform,
};

const SIZE: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct Headless {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn headless() -> Option<Headless> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("headless"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
    }))
    .ok()?;
    Some(Headless { device, queue })
}

/// One triangle over the middle of the viewport, facing the camera, leaving the corners clear.
fn quad(color: [f32; 3]) -> MeshData {
    let mut mesh = MeshData::default();
    for p in [[-0.6, -0.6, 0.0], [0.6, -0.6, 0.0], [0.0, 0.6, 0.0]] {
        mesh.push_vertex(p, [0.0, 0.0, 1.0]);
        mesh.colors.extend(color);
    }
    mesh.uvs.extend([0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);
    mesh
}

fn read_back(h: &Headless, target: &wgpu::Texture) -> RgbaImage {
    // Copies out through a buffer padded to the 256-byte row alignment wgpu requires.
    let row = (SIZE * 4).div_ceil(256) * 256;
    let buffer = h.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read back"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = h
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    h.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    h.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll");
    let mapped = slice.get_mapped_range().expect("mapped");
    let mut image = RgbaImage::new(SIZE, SIZE);
    for y in 0..SIZE {
        let start = (y * row) as usize;
        let line = &mapped[start..start + (SIZE * 4) as usize];
        image.data[(y * SIZE * 4) as usize..((y + 1) * SIZE * 4) as usize].copy_from_slice(line);
    }
    drop(mapped);
    buffer.unmap();
    image
}

/// Draws `mesh` with `material` into a black `SIZE` × `SIZE` target and reads the target back.
fn render_once(h: &Headless, renderer: &Renderer, material: &Material, mesh: &Mesh) -> RgbaImage {
    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let target = h.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = h
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = h
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        renderer.draw(&mut pass, material, mesh);
    }
    h.queue.submit([encoder.finish()]);
    read_back(h, &target)
}

#[test]
fn one_frame_renders_the_prototype_lambert_formula() {
    let Some(h) = headless() else {
        eprintln!("headless render skipped: no wgpu adapter (CI installs lavapipe)");
        return;
    };
    let mut renderer = Renderer::new(&h.device, &h.queue, FORMAT);

    // A texel the material samples, and a vertex colour, so the whole diffuse chain is exercised:
    // diffuse = material colour x vertex colour x texel.
    let mut image = RgbaImage::new(1, 1);
    image.put(0, 0, [200, 100, 50, 255]);
    let map = renderer.upload_texture(
        &h.device,
        &h.queue,
        &Texture {
            image,
            wrap_s: Wrap::Clamp,
            wrap_t: Wrap::Clamp,
            filter: Filter::Nearest,
            repeat: [1.0, 1.0],
        },
    );
    let mesh = renderer.upload_mesh(&h.device, &quad([0.5, 1.0, 1.0]));
    let material = renderer.material(
        &h.device,
        MaterialDesc::lambert(),
        &DrawUniform {
            color: [1.0, 1.0, 1.0, 1.0],
            flags: FLAG_TEXTURED | FLAG_VERTEX_COLOR,
            ..DrawUniform::default()
        },
        Some(&map),
    );
    // Identity camera: the quad already sits in clip space, and the normal faces +z.
    let scene = SceneUniform {
        view_proj: IDENTITY,
        ..SceneUniform::default()
    };
    renderer.set_scene(&h.queue, &scene);

    let out = render_once(&h, &renderer, &material, &mesh);

    // The same formula on the CPU: diffuse x (ambient + saturate(dot(n, l)) x sun).
    let scene = SceneUniform::default();
    let expected = |i: usize, texel: f32, vertex: f32| {
        let diffuse = texel / 255.0 * vertex;
        let lit = diffuse * (scene.ambient[i] + scene.sun_dir[2].max(0.0) * scene.sun_color[i]);
        (lit * 255.0).round() as i32
    };
    let got = out.get(SIZE / 2, SIZE / 2);
    for (i, (texel, vertex)) in [(200.0, 0.5), (100.0, 1.0), (50.0, 1.0)].iter().enumerate() {
        let want = expected(i, *texel, *vertex);
        assert!(
            (i32::from(got[i]) - want).abs() <= 1,
            "channel {i}: rendered {}, formula says {want}",
            got[i]
        );
    }
    assert_eq!(got[3], 255, "an opaque material leaves the target's alpha");
    assert_eq!(
        out.get(0, 0),
        [0, 0, 0, 255],
        "outside the triangle the clear stands"
    );
}

#[test]
fn a_screen_space_draw_ignores_the_camera() {
    let Some(h) = headless() else {
        eprintln!("screen-space render skipped: no wgpu adapter (CI installs lavapipe)");
        return;
    };
    let mut renderer = Renderer::new(&h.device, &h.queue, FORMAT);
    let mesh = renderer.upload_mesh(&h.device, &quad([0.5, 1.0, 1.0]));
    let material = renderer.material(
        &h.device,
        MaterialDesc::space(),
        &DrawUniform {
            flags: FLAG_SCREEN | FLAG_VERTEX_COLOR,
            ..DrawUniform::default()
        },
        None,
    );
    // A camera that collapses everything it touches to a point: whatever survives went round it.
    renderer.set_scene(
        &h.queue,
        &SceneUniform {
            view_proj: [0.0; 16],
            ..SceneUniform::default()
        },
    );
    let out = render_once(&h, &renderer, &material, &mesh);
    let got = out.get(SIZE / 2, SIZE / 2);
    for (i, want) in [128, 255, 255].iter().enumerate() {
        assert!(
            (i32::from(got[i]) - want).abs() <= 1,
            "channel {i}: rendered {}, expected {want}",
            got[i]
        );
    }
    assert_eq!(
        out.get(0, 0),
        [0, 0, 0, 255],
        "the backdrop still stops at its own triangle"
    );
}

#[test]
fn one_pipeline_per_distinct_material() {
    let Some(h) = headless() else {
        eprintln!("pipeline cache test skipped: no wgpu adapter (CI installs lavapipe)");
        return;
    };
    let mut renderer = Renderer::new(&h.device, &h.queue, FORMAT);
    let uniform = DrawUniform::default();
    for desc in [
        MaterialDesc::lambert(),
        MaterialDesc::lambert(),
        MaterialDesc::lambert_double_sided(),
        MaterialDesc::unlit(),
        MaterialDesc::border(),
        MaterialDesc::halo(),
        MaterialDesc::space(),
    ] {
        let _ = renderer.material(&h.device, desc, &uniform, None);
    }
    assert_eq!(
        renderer.pipeline_count(),
        6,
        "the repeated lambert reuses its pipeline"
    );
}
