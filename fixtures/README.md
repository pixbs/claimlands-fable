# fixtures

Golden values extracted from `reference/hex-planet.html` by `reference/harness/extract.mjs`. Tests
in `crates/*/tests/fixtures.rs` replay them. Do not edit by hand; regenerate with
`cargo xtask fixtures` and review the diff.

| Path | Holds | Consumed by |
|---|---|---|
| `index.json` | harness metadata, reference SHA-256, worlds extracted, section index | humans |
| `constants.json` | every top-level prototype constant | `cl-pixelart`, `cl-scenery` constant tests |
| `noise/hash3i.json`, `hash2.json`, `vnoise3.json`, `fbm3.json`, `mulberry32.json` | input → output vectors | `cl-noise` |
| `noise/js-semantics.json` | `ToInt32`, `Math.round`, `toFixed(6)`, `Math.hypot`, `Math.imul`, `>>>`, V8 `sin`/`cos`/`atan2`/`acos`/`pow`/`sqrt` | `cl-noise` |
| `hexsphere/icosahedron.json`, `n{2..12}.json` | counts, pentagons, `around`, neighbours, edge neighbours, corner tiles, quantised coordinate hashes; full coordinates and geodesic for `n ≤ 6` | `cl-hexsphere` |
| `worldgen/n{4,8}-s*.json` | level and cover per tile, land count, cover counts | `cl-worldgen`, `cl-hexsphere` (levels for frames) |
| `pixelart/cliff.*`, `foam.*`, `field.*`, `glow.*` | strip textures as PNG plus sampler settings | `cl-pixelart` |
| `pixelart/atlas-<world>.png/.json` | ground atlas, cell layout, coast/shoal/built fields | `cl-pixelart` (M1) |
| `pixelart/sky-s3521-deck{0,1,2}.png`, `sky-s3521.json` | cloud decks with Bayer alpha | `cl-pixelart` (M1) |
| `scenery/<world>-frames.json` | per-tile frames, facet planes, texel directions | `cl-hexsphere` |
| `scenery/<world>-terrain.json` | terrain fans, walls, surf, edge ribbons, `faceTile`/`wallTile` | `cl-scenery` (M1) |
| `scenery/<world>-cover.json` | fields (parcels, posts), forest, houses summaries; `coverLift` per tile | `cl-scenery` (M1) |
| `scenery/<world>-sky.json` | atmosphere and cloud shell | `cl-scenery` |
| `scenery/clouds-n8.json` | deck scales, materials, injected shader and uniforms | `cl-render` (M1) |
| `scenery/n4-s31676-border.json`, `-ring.json` | territory outline ribbons, hover ring | `cl-scenery` (M1) |
| `scenery/space-<W>x<H>.json` | vignette and star quads with flicker states | `cl-scenery` (M1) |

Mesh attributes are summarised as length, `sha256` of the little-endian `f32` bytes, `sha256` of the
values quantised to 1e-6, the first 64 floats and a stride-97 sample; `--full` dumps of the complete
arrays go to `reference/harness/out/` for debugging. Worlds: `n4-s31676`, `n4-s1234`, `n8-s63352`
(`31676 = 4 × 7919` and `63352 = 8 × 7919` are the prototype's default seeds for those sizes).
