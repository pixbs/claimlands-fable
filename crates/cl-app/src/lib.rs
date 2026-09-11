//! The application shell every platform boots into: winit window, GPU context, egui layer.
//! Platform entry points live in [`platform`].

mod app;
pub mod camera;
pub mod glow;
pub mod planet;
pub mod platform;
mod time;

pub use app::App;
pub use camera::{Camera, Trackball};
pub use glow::{Halo, Placement, placement};
pub use planet::Planet;

/// Build identifier shown in the readout.
pub const BUILD: &str = concat!("claimlands ", env!("CARGO_PKG_VERSION"));
