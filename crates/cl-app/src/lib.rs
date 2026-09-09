//! The application shell every platform boots into: winit window, GPU context, egui layer.
//! Platform entry points live in [`platform`].

mod app;
pub mod platform;

pub use app::App;

/// Build identifier shown in the readout.
pub const BUILD: &str = concat!("claimlands ", env!("CARGO_PKG_VERSION"));
