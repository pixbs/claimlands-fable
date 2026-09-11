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
| `make_cloud_sky`, `Sky`, `CloudDeck`, `CLOUD_DECKS`, `sky_seed` | `makeCloudSky`, its `buildClouds` call site | ported |
| `make_glow`, `GLOW_OUT` / `GLOW_BACK` / `GLOW_MAX` | `makeGlow` | ported |

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
- The sky runs on its own stream: `sky_seed` folds the world seed to `(seed % 9973) + 7`, as the
  prototype's call site does, so weather does not correlate with the terrain grown from the same
  number. Feeding `make_cloud_sky` a raw world seed gives a valid sky of the wrong world.
- The sky is a function of the seed alone; its size never is. One sky is shared by every world
  size, so `CLOUD_TEX_W` is fixed and the height follows from the equal-area aspect (`W / π`,
  rounded to a multiple of four).
- A deck's texture is white throughout: the tint is the material's, and alpha carries the Bayer
  rank so the shader dissolves a deck by lowering opacity rather than by repainting.
- Deck coverage is a quantile of the texels, and the grid is equal-area, so a deck's covered share
  equals its stated `cover`.
- The halo is the one texture sampled linearly rather than nearest: it is a glow, not pixel art, and
  nearest bands its gradient into rings. Its colour is flat `AIR_COLOR` throughout and the whole
  shape lives in the alpha channel.

## Testing
`tests/fixtures.rs` decodes `fixtures/pixelart/*.png` and compares texel bytes. Every strip is
exact, the ones sampling `hash2` (cliff, foam) included, and so are the ground atlas of all three
prototype worlds together with its per-tile and per-corner coast fields, and all three cloud decks
of `sky-s3521`, and the halo against `glow.png` together with the sampler and material rows of
`glow.json`. The atlas test builds its snapshot from `fixtures/worldgen/*.json`, so it pins the
port against the prototype's own levels and cover rather than against `cl-worldgen`. `src/atlas.rs`
covers `refresh` and `repaint` against a full rebuild; `src/sky.rs` covers the properties the
fixture cannot state — that the decks terrace inward and each covers its stated share of sky.

## Non-goals
Meshes and UVs (`cl-scenery`), GPU upload (`cl-render`). For the sky that means the cloud shells,
the drift and the see-through hole: this crate draws the decks, it does not place or animate them.
