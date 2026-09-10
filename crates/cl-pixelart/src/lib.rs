//! CPU pixel-art texture synthesis from the prototype's constants and a seed: strips, ground
//! atlas, cloud sky, halo. Sections 2, 3b, 4b (texture), clouds and glow of `hex-planet.html`.
#![forbid(unsafe_code)]

mod atlas;
pub mod palette;
mod sky;
mod strips;

pub use atlas::{
    Atlas, COAST_DARKEN, COAST_TIGHT, CoastFields, DITHER_LATTICE, GRASS_DITHER, GRASS_F0,
    GRASS_OCT, MUD_DITHER, MUD_EDGE, MUD_F0, MUD_OCT, MUD_SALT, MUD_SCATTER, SEA_DITHER, SEA_F0,
    SEA_FADE, SEA_OCT, SEA_SHALLOW, SPECKLE, build_terrain_atlas,
};
pub use cl_model::{Filter, Texture, Wrap};
pub use sky::{
    BAYER4, CLOUD_DECKS, CLOUD_F0, CLOUD_OCT, CLOUD_TEX_W, CloudDeck, DITHER_FLOOR, DITHER_RANKS,
    Sky, make_cloud_sky,
};
pub use strips::{
    BEACH_PX, CLIFF_H, CLIFF_W, FIELD_ROWS, FOAM_FRAMES, FOAM_H, FOAM_PX, FOAM_W, FURROW_PX,
    field_row_v, make_cliff_texture, make_field_texture, make_foam_texture,
};
