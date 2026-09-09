# cl-model

## Purpose
The vocabulary every other crate shares: tile ids, factions, terrain/cover/unit enums, the `WorldSnapshot`
the view layer reads, and the plain `MeshData` / `RgbaImage` containers the procedural builders emit.
It holds no logic beyond validation and conversion, so it never changes for feature reasons and pulls no
dependency beyond `serde`.

## Public API
| Item | Role |
|---|---|
| `TileId` | Index of a tile on the hex sphere; stable for a given frequency (pinned by `fixtures/hexsphere`) |
| `Faction` | `Red`, `Yellow`, `Green`, `Blue` in turn order; `letter()` gives the level-string code |
| `Terrain`, `Cover`, `UnitKind`, `UnitView`, `TileState` | Per-tile facts as the view needs them |
| `WorldSnapshot` | Frequency, seed and one `TileState` per tile; the only input `cl-scenery` takes |
| `MeshData` | Non-indexed triangle list: positions, normals, optional uvs and colours, optional per-triangle tile map |
| `RgbaImage` | CPU texture bytes with bounds-checked `put`/`get` |
| `world` | Scale constants copied from the prototype (`RADIUS`, `TILE_PX`, `UV_INSET`, `LEVEL_PX`, `ATMO_PX`, `CLOUD_PX`) and `tile_count(n)` |
| `hex_rgb` | `#rrggbb` to bytes, the prototype's `rgb()` |

## Invariants
- `tile_count(n) == 10n² + 2` for `2 ≤ n ≤ 12`; a valid `WorldSnapshot` holds exactly that many tiles.
- `MeshData::validate`: `positions.len() == normals.len()`, both multiples of 9; `uvs` empty or `2/3` of positions; `colors` empty or equal to positions; `face_tile` empty or one per triangle.
- `TileState::validate`: sea tiles carry no cover, owner or unit; `Capital` requires an owner (a `Town` may be neutral).

## Testing
Unit tests for the invariants and the letter/index round trips. No fixtures: nothing here computes.

## Non-goals
Game rules, geometry, palettes beyond `hex_rgb`, and any renderer or engine type.
