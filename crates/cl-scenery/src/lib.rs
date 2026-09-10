//! Procedural meshes from a world snapshot: the visible planet as plain `MeshData`.
#![forbid(unsafe_code)]

mod forest;
mod shells;
mod terrain;
mod zones;

pub use forest::{
    BUSH_CHANCE, BUSH_MARGIN_PX, CANOPY_BODY, CANOPY_ZONE_F, CANOPY_ZONES, CROWN_JIT, CROWN_PX,
    CROWN_STEP, FLOOR_GROW, FLOOR_LIFT_PX, FLOOR_R_MIN, FLOOR_SHADE, FOREST_SPAN, Forest,
    TREE_H_PX, TREE_MAX, TREE_MIN, TREE_SINK_PX, VIGOUR_F, build_forest,
};
pub use shells::{Shell, build_atmosphere, build_cloud_shell};
pub use terrain::{EDGE_HALF_PX, EDGE_LIFT, FOAM_LIFT, FOAM_V, Terrain, build_terrain};
