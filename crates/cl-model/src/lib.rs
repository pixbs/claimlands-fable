//! Shared vocabulary for Claim Lands: tile ids, factions, tile state, world snapshots, the `Board`
//! adjacency trait, mesh and texture containers. No logic beyond validation; no dependency beyond
//! `serde`.
#![forbid(unsafe_code)]

mod board;
mod faction;
mod image;
mod mesh;
mod tile;
pub mod world;

pub use board::{Board, GraphBoard};
pub use faction::Faction;
pub use image::{Filter, RgbaImage, Texture, Wrap, hex_rgb};
pub use mesh::MeshData;
pub use tile::{Cover, Terrain, TileId, TileState, UnitKind, UnitView, WorldSnapshot};
