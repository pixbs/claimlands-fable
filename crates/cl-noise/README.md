# cl-noise

## Purpose
The scalar building blocks every generator samples: the two hashes, 3D value noise, fractal Brownian
motion, the `mulberry32` generator, a 3-vector kit, and the JavaScript number semantics
(`ToInt32`, `Math.round`, `Math.hypot`, `toFixed`, `Math.sin`, `Math.cos`, `Math.pow`) the prototype
leans on. All of it is ported operation-for-operation so that a world generated on iOS, Android, the
web or CI is identical to the prototype's, bit for bit.

## Public API
| Item | Prototype | Notes |
|---|---|---|
| `hash2(x, y)` | `hash2` | `sin`-based; uses `js::sin` |
| `hash3i(x, y, z)`, `hash3(x, y, z)` | `hash3i` | integer hash; `hash3` applies `ToInt32` to doubles first, as `Math.imul` does |
| `vnoise3`, `fbm3`, `FBM_OCT`, `FBM_F0` | `vnoise3`, `fbm3` | `fbm3` treats `oct == 0` / `f0 == 0` as "use default", like `oct||FBM_OCT` |
| `Mulberry32` | `mulberry32` | seed goes through `ToInt32`; `next_f64()` in `[0, 1)` |
| `vec::{V3, add, sub, mul, dot, cross, len, norm}` | `V` | `len` is V8's `Math.hypot`; `norm` divides by 1 when the length is 0 or NaN |
| `js::{to_int32, imul, ushr, round, floor_half_up, hypot, hypot2, hypot3, to_fixed6}` | JS runtime | `round` is ties-toward-+∞, not Rust's `f64::round` |
| `js::{sin, cos, pow}` | `Math.sin`, `Math.cos`, `Math.pow` | V8's fdlibm, ported operation for operation; `libm`'s three round differently |

## Invariants
- Every function is pure and allocation-free except `to_fixed6`.
- No `std` transcendental function is called (clippy `disallowed-methods`); `sin`, `cos` and `pow`
  come from `js`, the rest from `libm`.
- `hash3i` and `Mulberry32` are integer-exact; the fixture tests compare bits, not tolerances.

## Testing
`tests/fixtures.rs` replays `fixtures/noise/*.json` extracted from the prototype by
`reference/harness/extract.mjs`: hashes, noise, fBm, mulberry32 sequences, and the JS-semantics table
(`ToInt32`, `Math.round`, `toFixed(6)`, `Math.hypot`, `Math.imul`, `>>>`, `sin`/`cos`/`atan2`/`acos`/`pow`
against V8, arguments past 2^19·π/2 included). All comparisons are exact.

## Non-goals
Geometry, textures, gameplay randomness (`cl-rules` uses a seeded `rand_xoshiro`), and any SIMD or
platform-specific fast path.
