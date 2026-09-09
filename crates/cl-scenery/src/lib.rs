//! Procedural meshes from a world snapshot: the visible planet as plain `MeshData`.
#![forbid(unsafe_code)]

mod shells;

pub use shells::{Shell, build_atmosphere, build_cloud_shell};
