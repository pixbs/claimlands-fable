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
| `build_terrain_atlas` with `refresh` / `repaint` | `buildTerrainAtlas` | issue M1 |
| `make_cloud_sky` | `makeCloudSky` | issue M1 |
| `make_glow` | `makeGlow` | issue M1 |

## Invariants
- Output is a function of constants and the seed only; no time, no platform calls.
- Texel values are integers in `0..=255` written exactly as the prototype computes them
  (`Math.round` semantics via `cl_noise::js::round`).
- Constants equal `fixtures/constants.json` (tested).

## Testing
`tests/fixtures.rs` decodes `fixtures/pixelart/*.png` and compares texel bytes. Strips using
`hash2` (cliff, foam) allow the documented tolerance budget until the fdlibm `sin` port lands; the
field strip is exact.

## Non-goals
Meshes and UVs (`cl-scenery`), GPU upload (`cl-render`).
