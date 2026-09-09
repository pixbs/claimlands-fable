//! The winit application: window, asynchronous GPU start-up, resize, redraw and the egui layer.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cl_hexsphere::HexSphere;
use cl_model::WorldSnapshot;
use cl_render::Gpu;
use cl_ui::{Hud, HudAction, HudInfo};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

/// The prototype's clear colour behind the space pass (`SKY_RIM`), as raw 0–1 values.
const CLEAR: [f64; 3] = [3.0 / 255.0, 2.0 / 255.0, 9.0 / 255.0];
/// Default pixel scale (`pixelScale = 3` in the prototype).
const PIXEL_SCALE: u32 = 3;

/// egui state bound to one window and one surface format.
struct EguiLayer {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

/// Application state shared with the asynchronous GPU start-up on the web.
pub struct App {
    window: Option<Arc<Window>>,
    gpu: Rc<RefCell<Option<Gpu>>>,
    egui: Option<EguiLayer>,
    hud: Hud,
    world: WorldSnapshot,
    sphere: HexSphere,
    attributes: WindowAttributes,
}

impl App {
    /// An app showing the default world (`n = 8`, the prototype's default seed for that size).
    pub fn new(attributes: WindowAttributes) -> Self {
        let world = cl_worldgen::generate_world(8, 8 * 7919);
        let sphere = HexSphere::build(8);
        Self {
            window: None,
            gpu: Rc::new(RefCell::new(None)),
            egui: None,
            hud: Hud::default(),
            world,
            sphere,
            attributes,
        }
    }

    fn hud_info(&self, gpu: &Gpu) -> HudInfo {
        HudInfo {
            tiles: self.world.tiles.len(),
            land: self
                .world
                .tiles
                .iter()
                .filter(|t| t.terrain.is_land())
                .count(),
            pentagons: self.sphere.pentagons().len(),
            frequency: self.world.frequency,
            seed: self.world.seed,
            pixel_scale: gpu.pixel_scale(),
            target: gpu.target_extent(),
            build: crate::BUILD,
        }
    }

    fn ensure_egui(&mut self, window: &Window, gpu: &Gpu) {
        if self.egui.is_some() {
            return;
        }
        let ctx = egui::Context::default();
        cl_ui::theme::apply(&ctx);
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.surface_format(),
            egui_wgpu::RendererOptions::default(),
        );
        self.egui = Some(EguiLayer {
            ctx,
            state,
            renderer,
        });
    }

    fn redraw(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let gpu_cell = self.gpu.clone();
        let mut guard = gpu_cell.borrow_mut();
        let Some(gpu) = guard.as_mut() else { return };
        self.ensure_egui(&window, gpu);
        let info = self.hud_info(gpu);

        let Some(mut frame) = gpu.frame() else { return };
        frame.clear_target(CLEAR);
        frame.blit();

        let egui = self.egui.as_mut().expect("created above");
        let raw_input = egui.state.take_egui_input(&window);
        let hud = &mut self.hud;
        let mut action = HudAction::None;
        let output = egui.ctx.run_ui(raw_input, |ctx| {
            action = hud.ui(ctx, &info);
        });
        egui.state
            .handle_platform_output(&window, output.platform_output);
        let clipped = egui.ctx.tessellate(output.shapes, output.pixels_per_point);
        let (w, h) = gpu.surface_size();
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [w, h],
            pixels_per_point: output.pixels_per_point,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                egui.renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let extra = egui.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            frame.encoder(),
            &clipped,
            &screen,
        );
        {
            let view = frame.surface_view().clone();
            let mut pass = frame
                .encoder()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            egui.renderer.render(&mut pass, &clipped, &screen);
        }
        for id in &output.textures_delta.free {
            egui.renderer.free_texture(id);
        }
        gpu.queue.submit(extra);
        frame.finish();

        if let HudAction::SetPixelScale(s) = action {
            gpu.set_pixel_scale(s);
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(self.attributes.clone())
                .expect("window"),
        );
        let size = window.inner_size();
        let scale = window.scale_factor();
        self.window = Some(window.clone());
        let slot = self.gpu.clone();
        let init = async move {
            match Gpu::new(window.clone(), size.width, size.height, scale, PIXEL_SCALE).await {
                Ok(gpu) => {
                    *slot.borrow_mut() = Some(gpu);
                    window.request_redraw();
                }
                Err(e) => log::error!("GPU start-up failed: {e}"),
            }
        };
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(init);
        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(init);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let (Some(window), Some(egui)) = (self.window.as_ref(), self.egui.as_mut()) {
            let response = egui.state.on_window_event(window, &event);
            if response.repaint {
                window.request_redraw();
            }
            if response.consumed {
                return;
            }
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                if let Some(gpu) = self.gpu.borrow_mut().as_mut() {
                    gpu.resize(size.width, size.height, scale);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = self.window.as_ref().map(|w| w.inner_size());
                if let (Some(gpu), Some(size)) = (self.gpu.borrow_mut().as_mut(), size) {
                    gpu.resize(size.width, size.height, scale_factor);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}
