//! Procedural meshes from a world snapshot: the visible planet as plain `MeshData`.
#![forbid(unsafe_code)]

mod fields;
mod poly;
mod shells;
mod terrain;
mod zones;

pub use fields::{
    FARM_SPAN, FIELD_MIN_PX2, FIELD_SALT, FIELD_TRIM, Fields, PARCEL_MAX, PARCEL_MIN, POST_CHANCE,
    POST_SIDE, POST_STEP, POST_TALL, POST_TOP, POST_W, SLIVER_PX2, THIN_PX, build_fields,
};
pub use poly::{
    Poly, clip_half, clip_to_hull, dedupe, hull_at, mitre_offset, parcel_split, poly_area,
    poly_thickness, trim_convex,
};
pub use shells::{Shell, build_atmosphere, build_cloud_shell};
pub use terrain::{EDGE_HALF_PX, EDGE_LIFT, FOAM_LIFT, FOAM_V, Terrain, build_terrain};
pub use zones::{ZoneFrame, Zones, cover_zones, ring_normal, zone_frame};
