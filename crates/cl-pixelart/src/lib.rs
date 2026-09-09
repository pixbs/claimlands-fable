//! CPU pixel-art texture synthesis from the prototype's constants and a seed: strips, ground
//! atlas, cloud sky, halo. Sections 2, 3b, 4b (texture), clouds and glow of `hex-planet.html`.
#![forbid(unsafe_code)]

pub mod palette;
mod strips;

pub use strips::{
    BEACH_PX, CLIFF_H, CLIFF_W, FIELD_ROWS, FOAM_FRAMES, FOAM_H, FOAM_PX, FOAM_W, FURROW_PX,
    Texture, Wrap, field_row_v, make_cliff_texture, make_field_texture, make_foam_texture,
};
