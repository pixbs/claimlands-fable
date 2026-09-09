# cl-scenery

## Purpose
Everything visible on and around the planet as plain `MeshData`, computed from a `WorldSnapshot`,
the hex sphere and its frames. Each builder is a pure function; the app decides when to rebuild
which one. Ported from prototype sections 4, 4b, 4c, 4d, the cloud shell, the atmosphere, the
territory outline and the space pass.

## Public API
| Builder | Prototype | Status |
|---|---|---|
| `build_atmosphere(&sphere, px)` | `buildAtmosphere` | ported |
| `build_cloud_shell(&sphere, &frames, px)` | `buildCloudShell` | ported |
| `build_terrain(&sphere, &frames, &snapshot, &atlas)` -> `Terrain` (fans, cliff wedges, surf, edge ribbons, `face_tile`, `wall_tile`, `vertex_start`/`vertex_count`) | `buildMesh` | ported |
| `build_fields` (zones, parcels, fences) | `buildFields` | issue M1 |
| `build_forest` | `buildForest` | issue M1 |
| `build_houses` | `buildHouses` | issue M1 |
| `build_border`, `hover_ring` | `rebuildBorder`, `setRing` | issue M1 |
| `build_space(w, h)` (vignette + stars), `step_stars` | `buildSpace`, `stepStars` | issue M1 |
| `pick(ray, …)` | raycast to tile via `faceTile` / `wallTile` | issue M1 |
| `Shell` | radius plus mesh, for shells whose radius the app needs | ported |

## Invariants
- Builders read `WorldSnapshot`, never `GameState`.
- Positions are `f64` until the final `f32` push; triangle order is the prototype's, so the
  `sha256_f32` of a builder's output equals the fixture's.
- Every mesh passes `MeshData::validate()`. The edge ribbons carry positions only: they are drawn
  unlit and need no normals.
- Corner displacement along a coast is a property of the corner, not of the tile asking for it, so
  both tiles meeting there place the wedge foot at the same point and the coastline cannot crack.

## Testing
`tests/fixtures.rs` compares each ported builder with `fixtures/scenery/*.json`: vertex counts,
the exact `sha256` of the little-endian `f32` bytes, and the first 64 floats for readable diffs.

## Non-goals
GPU resources, materials and shaders (`cl-render`), gameplay.
