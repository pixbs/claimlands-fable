//! The winit application: window, asynchronous GPU start-up, resize, redraw and the egui layer.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cl_hexsphere::HexSphere;
use cl_model::WorldSnapshot;
use cl_render::{Gpu, Renderer, SceneUniform};
use cl_ui::{Hud, HudAction, HudInfo};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::camera::{Camera, Trackball, spread};
use crate::planet::Planet;
use crate::space::Backdrop;
use crate::time::now_ms;

/// The prototype's clear colour behind the space pass ([`cl_scenery::SKY_RIM`]), as raw 0–1 values.
const CLEAR: [f64; 3] = [3.0 / 255.0, 2.0 / 255.0, 9.0 / 255.0];
/// Pointer id of the mouse; touches carry their own.
const MOUSE: u64 = u64::MAX;
/// Default pixel scale (`pixelScale = 3` in the prototype).
const PIXEL_SCALE: u32 = 3;

/// The size to configure the surface with once the GPU is ready, given what the window reported
/// when it was created and what it reports now.
///
/// On the web the canvas has no layout when the window is created, so the first of those is
/// `0 × 0`, and the `Resized` carrying the real size arrives while [`Gpu::new`] is still awaiting
/// the adapter — with no GPU to hand it to. winit then stays quiet until the window changes size
/// again, so reading the window once more at the end of start-up is what keeps the surface off
/// `1 × 1`. The size at creation only stands in while the window still has none of its own.
fn startup_size(at_creation: (u32, u32), now: (u32, u32)) -> (u32, u32) {
    if now.0 > 0 && now.1 > 0 {
        now
    } else {
        at_creation
    }
}

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
    renderer: Option<Renderer>,
    planet: Option<Planet>,
    backdrop: Option<Backdrop>,
    camera: Camera,
    trackball: Trackball,
    input: Input,
    cursor: (f32, f32),
}

/// Pointers currently down, and what the gesture they are making has done so far.
#[derive(Debug, Default)]
struct Input {
    /// Id and last position of every pointer down, in insertion order.
    pointers: Vec<(u64, (f32, f32))>,
    dragging: bool,
    /// Total travel of the gesture: under [`TAP_SLOP`] it was a tap.
    moved: f32,
    /// Finger spread and camera distance when a pinch began.
    pinch_start: f32,
    cam_start: f32,
}

impl Input {
    fn press(&mut self, id: u64, at: (f32, f32), camera_distance: f32) {
        if let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) {
            p.1 = at;
        } else {
            self.pointers.push((id, at));
        }
        if self.pointers.len() == 2 {
            self.pinch_start = spread(self.pointers[0].1, self.pointers[1].1);
            self.cam_start = camera_distance;
        }
        self.dragging = true;
        self.moved = 0.0;
    }

    /// The movement of pointer `id`, or `None` when it is not down.
    fn moved_to(&mut self, id: u64, at: (f32, f32)) -> Option<(f32, f32)> {
        let p = self.pointers.iter_mut().find(|p| p.0 == id)?;
        let delta = (at.0 - p.1.0, at.1 - p.1.1);
        p.1 = at;
        self.moved += delta.0.abs() + delta.1.abs();
        Some(delta)
    }

    fn release(&mut self, id: u64) {
        self.pointers.retain(|p| p.0 != id);
        if self.pointers.is_empty() {
            self.dragging = false;
        }
        self.pinch_start = 0.0;
    }
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
            renderer: None,
            planet: None,
            backdrop: None,
            camera: Camera::default(),
            trackball: Trackball::default(),
            input: Input::default(),
            cursor: (0.0, 0.0),
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

    /// Physical pixels to the logical (CSS) pixels the prototype measures drags in.
    fn to_logical(&self, x: f64, y: f64) -> (f32, f32) {
        let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
        ((x / scale) as f32, (y / scale) as f32)
    }

    fn pointer_pressed(&mut self, id: u64, at: (f32, f32)) {
        self.input.press(id, at, self.camera.distance());
        self.trackball.hold();
    }

    /// One pointer moved. Two pointers down is a pinch; one is a drag.
    fn pointer_moved(&mut self, id: u64, at: (f32, f32)) {
        if id == MOUSE {
            self.cursor = at;
        }
        if !self.input.dragging {
            return;
        }
        let Some((dx, dy)) = self.input.moved_to(id, at) else {
            return;
        };
        if self.input.pointers.len() == 2 {
            let now = spread(self.input.pointers[0].1, self.input.pointers[1].1);
            self.camera
                .pinch(self.input.cam_start, self.input.pinch_start, now);
        } else {
            self.trackball.drag(dx, dy);
        }
    }
    /// Builds the renderer and uploads the planet, once the GPU exists.
    fn ensure_scene(&mut self, gpu: &Gpu) {
        if self.renderer.is_some() {
            return;
        }
        let mut renderer = Renderer::new(&gpu.device, &gpu.queue, gpu.target_format());
        self.planet = Some(Planet::new(
            &mut renderer,
            &gpu.device,
            &gpu.queue,
            self.sphere.clone(),
            self.world.clone(),
        ));
        let (w, h) = gpu.target_extent();
        self.backdrop = Some(Backdrop::new(&mut renderer, &gpu.device, w, h));
        self.renderer = Some(renderer);
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

        self.ensure_scene(gpu);

        // Inertia, idle drift and the surf all advance here, so one redraw is one frame.
        self.trackball.step(false);
        let (tw, th) = gpu.target_extent();
        let aspect = tw as f32 / th as f32;
        if let Some(renderer) = &self.renderer {
            renderer.set_scene(
                &gpu.queue,
                &SceneUniform {
                    view_proj: self.camera.view_proj(aspect),
                    ..SceneUniform::default()
                },
            );
        }
        if let Some(planet) = &mut self.planet {
            planet.animate(&gpu.queue, now_ms());
            planet.set_model(&gpu.queue, self.trackball.model());
            planet.set_camera_distance(&gpu.queue, self.camera.distance());
        }
        if let (Some(renderer), Some(backdrop)) = (&self.renderer, &mut self.backdrop) {
            backdrop.resize(renderer, &gpu.device, tw, th);
        }

        let Some(mut frame) = gpu.frame() else { return };
        {
            // The backdrop clears the colour and writes no depth, exactly as the prototype's
            // `renderer.clear()` and its space pass do. The clear is what shows through where the
            // vignette's own triangles have not covered a texel yet.
            let mut pass = frame.scene_pass(Some(CLEAR), false);
            if let (Some(renderer), Some(backdrop)) = (&self.renderer, &self.backdrop) {
                backdrop.draw(renderer, &mut pass);
            }
        }
        {
            // Then the depth clear, so the planet is never occluded by what is behind it.
            let mut pass = frame.scene_pass(None, true);
            if let (Some(renderer), Some(planet)) = (&self.renderer, &self.planet) {
                planet.draw(renderer, &mut pass);
            }
        }
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
        }
        // The surf steps and a flick decays on a clock, so the loop runs continuously, as the
        // prototype's requestAnimationFrame does.
        window.request_redraw();
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
                Ok(mut gpu) => {
                    // A `Resized` that arrived while this future was awaiting the adapter found
                    // no GPU to resize; asking the window again is what catches it.
                    let started_at = (size.width, size.height);
                    let latest = window.inner_size();
                    let size = startup_size(started_at, (latest.width, latest.height));
                    if size != started_at {
                        gpu.resize(size.0, size.1, window.scale_factor());
                    }
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
            WindowEvent::CursorMoved { position, .. } => {
                let logical = self.to_logical(position.x, position.y);
                self.pointer_moved(MOUSE, logical);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    match state {
                        ElementState::Pressed => self.pointer_pressed(MOUSE, self.cursor),
                        ElementState::Released => self.input.release(MOUSE),
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Only the sign counts, as in the prototype, and winit's positive y is a scroll
                // away from the user where the DOM's is toward it.
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.camera.wheel(-y);
            }
            WindowEvent::Touch(touch) => {
                let at = self.to_logical(touch.location.x, touch.location.y);
                let id = touch.id;
                match touch.phase {
                    winit::event::TouchPhase::Started => self.pointer_pressed(id, at),
                    winit::event::TouchPhase::Moved => self.pointer_moved(id, at),
                    winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                        self.input.release(id);
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn a_resize_during_gpu_start_up_is_not_lost() {
        // The web: no canvas layout when the window is created, the real size delivered while the
        // GPU was still starting. Taking the creation size here is what left the surface at 1x1.
        assert_eq!(startup_size((0, 0), (1350, 1270)), (1350, 1270));
        // Any platform: a resize that landed mid-start-up wins over the size at creation.
        assert_eq!(startup_size((1280, 720), (1000, 700)), (1000, 700));
        // A window that still reports nothing keeps what it was created at rather than collapsing.
        assert_eq!(startup_size((1280, 720), (0, 0)), (1280, 720));
        assert_eq!(startup_size((1280, 720), (1000, 0)), (1280, 720));
    }

    #[test]
    fn the_clear_is_the_rim_of_the_sky() {
        let rim = cl_model::hex_rgb(cl_scenery::SKY_RIM);
        assert_eq!(CLEAR, rim.map(|c| f64::from(c) / 255.0));
    }
}
