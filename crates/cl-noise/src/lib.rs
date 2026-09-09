//! Deterministic scalar math ported bit-for-bit from the prototype: hashes, value noise, fBm, the
//! `mulberry32` generator, a 3-vector kit, and the JavaScript number semantics they rely on. Every
//! function is pinned by `fixtures/noise`; `docs/design/porting.md` lists the rules that keep the
//! output identical on every platform.
#![forbid(unsafe_code)]

pub mod hash;
pub mod js;
pub mod rng;
pub mod vec;

pub use hash::{FBM_F0, FBM_OCT, fbm3, hash2, hash3, hash3i, vnoise3};
pub use rng::Mulberry32;
pub use vec::V3;
