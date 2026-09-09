//! Procedural meshes from a world snapshot: the visible planet as plain `MeshData`.
#![forbid(unsafe_code)]

mod shells;
mod terrain;

pub use shells::{Shell, build_atmosphere, build_cloud_shell};
pub use terrain::{EDGE_HALF_PX, EDGE_LIFT, FOAM_LIFT, FOAM_V, Terrain, build_terrain};
