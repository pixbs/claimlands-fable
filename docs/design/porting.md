# Porting rules

The prototype is JavaScript; the port is Rust that must produce the same bits. These rules make that
true on every platform. `clippy.toml` enforces the ones a lint can catch.

| Rule | Reason |
|---|---|
| Keep the prototype's expressions and evaluation order: `(a * w + b * i + c * j) / n` stays exactly that. | IEEE 754 arithmetic is deterministic only for the same operation sequence. |
| `f64` everywhere until the final `f32` push into `MeshData`. | JavaScript numbers are doubles; three.js converts to `Float32Array` at the end. |
| No FMA: never `mul_add`. | JavaScript never fuses; a fused multiply-add changes the last bit. |
| `Math.sin`, `Math.cos`, `Math.pow` → `cl_noise::js::{sin, cos, pow}`; the other transcendentals through `libm` (`libm::atan2`, `libm::acos`), never `f64::sin` & co. | Platform libms differ, and `libm`'s own `sin`/`cos`/`pow` round differently from V8 for ~1 % of inputs. `cl_noise::js` is V8's fdlibm ported operation for operation, so every platform lands on the prototype's bits. |
| Lengths through `cl_noise::vec::len` (V8's compensated `Math.hypot`), not `sqrt(x²+y²+z²)`. | V8's hypot scales and Kahan-sums; the naive form differs by an ulp often enough to move dithered texels. |
| `Math.round` → `cl_noise::js::round`; `x \| 0`, `Math.imul`, `>>>` → `to_int32`, `imul`, `ushr`. | Rust's `round` ties away from zero; JavaScript ties toward +∞ and truncates to int32 modulo 2³². |
| `toFixed(6)` → `cl_noise::js::to_fixed6` (exact decimal rounding). | The geodesic vertex key decides tile ids. |
| `mulberry32` and `hash3i` are the only random and hash sources in generation; consume them in the prototype's order, including short-circuited calls (`n.cover \|\| n.level < 0 \|\| rnd() > p`). | Every extra or missing draw shifts every later value. |
| `Map`/`Set` insertion order → `Vec` with contains-checks or `BTreeMap`; `HashMap` is banned. | Neighbour order and zone order are part of the output. |
| Sorting: `sort_by(partial_cmp)` (stable) where the prototype sorts; sort keys must be identical. | Different tie handling reorders corners and quantiles. |
| Integer-valued doubles stay `f64` where the prototype keeps them as numbers (`seed * 1.7`); convert with `to_int32` only where JavaScript would (`Math.imul` arguments). | Implicit conversions are where ports silently diverge. |
| JSON fixtures are parsed with `serde_json` `float_roundtrip`. | The default parser is not correctly rounded; it shifted inputs by an ulp and made `sqrt` "differ". |
| A deliberate deviation is commented at the site with the reason and covered by a fixture or test that documents the new value. | Silent deviations are indistinguishable from bugs. |

## Verifying a port

1. Extract or locate the fixture (`reference/harness/extract.mjs`, `fixtures/`).
2. Write the test first: exact comparison, all mismatches reported with inputs.
3. Port the function keeping the prototype's structure; name variables as the prototype does where
   that helps a reviewer line the two up.
4. On a mismatch, bisect inside the function: compare intermediate values against `node -e` runs
   of the same prototype expression.
