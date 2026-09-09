# Testing

## Layers

| Layer | Tool | Where | Catches |
|---|---|---|---|
| Unit tests per rule or function | `#[test]`, names carry rule ids (`r_eco_03_*`) | `crates/*/src` | logic errors |
| Fixture tests | JSON/PNG under `fixtures/`, extracted from the prototype | `crates/*/tests/fixtures.rs` | any deviation from the prototype |
| Property tests | `proptest` | `cl-level`, `cl-rules` | invariants under random input (round trips, transactional `apply`) |
| Snapshot tests | `insta` | scripted matches, level parses | regressions in behaviour that has no oracle |
| Compile targets | CI: host, `wasm32`, Android, iOS | `.github/workflows` | platform breakage |
| Visual regression (M1) | headless screenshot vs baseline PNG | CI | accidental look changes |
| Coverage floor | `cargo llvm-cov` on `cl-rules`, 80 % lines now, raised as rules land | CI | untested rules |

`cargo xtask check` runs the local layers; CI runs the same commands.

## Fixtures

`reference/harness/extract.mjs` runs the frozen prototype in Node and writes `fixtures/`:

| Directory | Content | Compared how |
|---|---|---|
| `noise/` | hash, noise, fBm, mulberry32 vectors; JavaScript semantics table; V8 transcendentals | bit-exact |
| `hexsphere/` | per frequency: counts, pentagons, neighbours, corner tiles, coordinates (`n ≤ 6` in full), quantised hashes | exact |
| `worldgen/` | levels and cover per tile for three worlds | exact |
| `pixelart/` | PNGs of every texture the prototype draws, plus sampler settings | texel-exact |
| `scenery/` | per builder: attribute lengths, `sha256` of the `f32` bytes, first 64 floats, a stride sample | exact hash |
| `constants.json` | every top-level prototype constant | exact |

Rules: never edit fixtures by hand; regenerate with `cargo xtask fixtures` only when the harness
changed; a fixture diff in a PR needs the `visual-change` label and a reason. The reference file is
frozen; its SHA-256 is recorded in `fixtures/index.json`.

## Tolerances

There are none. Every fixture comparison is exact: a single differing bit fails. Transcendentals
were the last exception — `libm` (musl lineage) rounds `sin`, `cos` and `pow` differently from V8's
fdlibm for about 1 % of arguments, which reached `hash2` and the strips built on it — and
`cl_noise::js::{sin, cos, pow}` now reproduces V8 bit for bit, so those rows are exact too. A new
tolerance needs a reason in the PR and a constant named after what it covers.

## Writing tests

- Name rule tests after the rule id: `fn r_bld_02_field_cost_grows_per_field()`.
- Rules tests use `GraphBoard::flower()` (seven tiles) or another tiny graph; no sphere needed.
- Fixture tests report all mismatches, not the first, and include the input in the message.
- A refused command must leave the state untouched: assert `state == before`.
- Snapshot updates go through `cargo insta review`; the diff is part of the PR.
