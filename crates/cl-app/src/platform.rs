//! Entry points per platform. Everything above this module is target-agnostic.

use winit::window::WindowAttributes;

/// Window attributes shared by every platform.
#[allow(dead_code)]
fn attributes() -> WindowAttributes {
    WindowAttributes::default().with_title("Claim Lands")
}

#[allow(unused_imports)]
use crate::App;

/// Web: install hooks, append a full-window canvas to the document and spawn the event loop.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    let attrs = attributes().with_append(true).with_prevent_default(true);
    event_loop.spawn_app(App::new(attrs));
}

/// Android: the `GameActivity` entry point.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    let event_loop = winit::event_loop::EventLoop::builder()
        .with_android_app(android_app)
        .build()
        .expect("event loop");
    let mut app = App::new(attributes());
    event_loop.run_app(&mut app).expect("event loop");
}

/// iOS: called from the Xcode project's `main.m`; never returns.
#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn claimlands_main() {
    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    let mut app = App::new(attributes());
    event_loop.run_app(&mut app).expect("event loop");
}
