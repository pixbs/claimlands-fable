# cl-worldgen

## Purpose
Turns a seed into a starting world: which tiles are land (prototype `generateTerrain`, lines
453–473) and where the initial villages, woods and farmland clumps lie (`seedCover`, lines
1101–1130). Both are the prototype's algorithms operation for operation, driven by the same
`mulberry32` stream, so a seed produces the same planet the reviewer sees at `/reference/`.

## Public API
| Item | Prototype | Notes |
|---|---|---|
| `generate_terrain(&sphere, seed)` | `generateTerrain` | one level per tile: `0` land, `-1` sea; seven directional waves, quantile cut at `LAND_FRACTION`, one smoothing pass |
| `seed_cover(&sphere, &levels, seed)` | `seedCover` | clumps of `Town`, `Forest`, `Field` covering `TOWN_SHARE`, `WOOD_SHARE`, `FARM_SHARE` of the land |
| `generate_world(n, seed)` | `makeWorld` (terrain and cover part) | a `WorldSnapshot` with no owners or units |
| `LAND_FRACTION`, `TOWN_SHARE`, `WOOD_SHARE`, `FARM_SHARE` | same names | shares of tiles |

## Invariants
- Deterministic: same `(n, seed)` → same output on every platform (fixtures pin three worlds).
- Cover only appears on land; sea tiles stay `Cover::None`.
- `seed == 0` is not special here; the level format gives it the meaning "no continents".

## Testing
`tests/fixtures.rs` compares levels and cover with `fixtures/worldgen/*.json` for `(4, 31676)`,
`(4, 1234)` and `(8, 63352)`, exact per tile.

## Non-goals
Turn-time forest growth (a rule in `cl-rules`), level strings, meshes, textures.
