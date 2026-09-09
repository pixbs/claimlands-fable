# cl-pixelart

## Purpose
Every texture the planet uses, drawn texel by texel on the CPU from the prototype's constants:
the cliff and surf strips, the field strip, the per-tile ground atlas, the equal-area cloud sky with
its Bayer alpha, and the halo. Nothing is loaded from disk; a seed is the only input. Ported from
prototype sections 2, 3b, 4b (texture), the cloud sky and the glow.

## Public API
| Item | Prototype | Status |
|---|---|---|
| `palette` | colour constants (`GRASS_BANDS`, `SEA_BANDS`, `MUD_BANDS`, `CLIFF_ROWS`, `FOAM_ROWS`, `AIR_COLOR`, `FIELD_CROPS` …) | ported |
| `Texture`, `Wrap` | `pixelTexture` settings | ported |
| `make_cliff_texture()` | `makeCliffTexture` | ported |
| `make_foam_texture()` | `makeFoamTexture` (8-frame sheet) | ported |
| `make_field_texture()`, `field_row_v` | `makeFieldTexture`, `fieldRowV` | ported |
| `build_terrain_atlas`, `Atlas::refresh` / `Atlas::repaint`, `CoastFields` | `buildTerrainAtlas` | ported |
| `make_cloud_sky` | `makeCloudSky` | issue M1 |
| `make_glow` | `makeGlow` | issue M1 |

## Invariants
- Output is a function of constants and the seed only; no time, no platform calls.
- Texel values are integers in `0..=255` written exactly as the prototype computes them
  (`Math.round` semantics via `cl_noise::js::round`).
- Constants equal `fixtures/constants.json` (tested).
- The atlas is a function of the sphere, the snapshot's levels and cover, and the seed. `refresh`
  (after a level change) and `repaint` (after a cover change) leave it byte-identical to a full
  rebuild; they exist only to avoid repainting the whole planet.
- A capital paints as a village: the prototype has only `houses`, and a capital walks the ground
  bare the same way.

## Testing
`tests/fixtures.rs` decodes `fixtures/pixelart/*.png` and compares texel bytes. Every strip is
exact, the ones sampling `hash2` (cliff, foam) included, and so is the ground atlas of all three
prototype worlds together with its per-tile and per-corner coast fields. The atlas test builds its
snapshot from `fixtures/worldgen/*.json`, so it pins the port against the prototype's own levels and
cover rather than against `cl-worldgen`. `src/atlas.rs` covers `refresh` and `repaint` against a
full rebuild.

## Non-goals
Meshes and UVs (`cl-scenery`), GPU upload (`cl-render`).
