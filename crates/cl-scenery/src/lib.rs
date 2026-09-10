//! Procedural meshes from a world snapshot: the visible planet as plain `MeshData`.
#![forbid(unsafe_code)]

mod fields;
mod forest;
mod houses;
mod poly;
mod shells;
mod terrain;
mod zones;

pub use fields::{
    FARM_SPAN, FIELD_MIN_PX2, FIELD_SALT, FIELD_TRIM, Fields, PARCEL_MAX, PARCEL_MIN, POST_CHANCE,
    POST_SIDE, POST_STEP, POST_TALL, POST_TOP, POST_W, SLIVER_PX2, THIN_PX, build_fields,
};
pub use forest::{
    BUSH_CHANCE, BUSH_MARGIN_PX, CANOPY_BODY, CANOPY_ZONE_F, CANOPY_ZONES, CROWN_JIT, CROWN_PX,
    CROWN_STEP, FLOOR_GROW, FLOOR_LIFT_PX, FLOOR_R_MIN, FLOOR_SHADE, FOREST_SALT, FOREST_SPAN,
    Forest, TREE_H_PX, TREE_MAX, TREE_MIN, TREE_SINK_PX, VIGOUR_F, build_forest,
};
pub use houses::{
    CHIM_ODDS, CHIM_PX, CHIM_RISE_MAX, CHIM_RISE_MIN, DOOR_H, DOOR_W, HOUSE_LEN_MAX, HOUSE_LEN_MIN,
    HOUSE_ODDS, HOUSE_OPEN, HOUSE_ROOF, HOUSE_ROT_JIT, HOUSE_SALT, HOUSE_SINK, HOUSE_SPAN,
    HOUSE_SPAN_MAX, HOUSE_SPAN_MIN, HOUSE_TURNS, HOUSE_WALLS, Houses, L_CHANCE, PLOT_JIT, PLOT_PX,
    RIDGE_CAP_PX, ROOF_LIP, ROOF_OVER, STRIPE_ODDS, STRIPE_PX, VERGE_ODDS, VERGE_PX, WALL_MAX,
    WALL_MIN, WIN_PX, build_houses,
};
pub use poly::{
    Poly, clip_half, clip_to_hull, dedupe, hull_at, mitre_offset, parcel_split, poly_area,
    poly_thickness, trim_convex,
};
pub use shells::{
    Clouds, Deck, HOLE_REST_IN, HOLE_REST_OPEN, HOLE_REST_OUT, Shell, build_atmosphere,
    build_cloud_shell, build_clouds, hole_rest,
};
pub use terrain::{EDGE_HALF_PX, EDGE_LIFT, FOAM_LIFT, FOAM_V, Terrain, build_terrain};
pub use zones::{ZoneFrame, Zones, cover_zones, ring_normal, zone_frame};
