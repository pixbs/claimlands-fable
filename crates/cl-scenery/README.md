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
| `build_fields(&sphere, &frames, &snapshot, px)` -> `Option<Fields>` (surface, posts, zone/parcel/fence counts) | `buildFields` | ported |
| `build_forest(&sphere, &frames, &snapshot, px, seed)` -> `Option<Forest>` (crowns and understory discs in one mesh, plus zone, tile and crown counts) | `buildForest` | ported |
| `cover_zones`, `zone_frame`, `ZoneFrame::{to2, to3, to2e, to3e}`, `ring_normal` | `coverZones`, `zoneFrame`, `ringNormal` | ported |
| `poly`: `clip_half`, `clip_to_hull`, `trim_convex`, `dedupe`, `mitre_offset`, `parcel_split`, `poly_area`, `poly_thickness`, `hull_at` | the 2D convex polygon kit | ported |
| `build_houses(&sphere, &frames, &snapshot, px, seed)` -> `Option<Houses>` (every house and wing in one mesh, plus zone, tile, house and wing counts) | `buildHouses` | ported |
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
- Cover is laid out per zone and only then cut along the hex borders, so a field straddling a tile
  border keeps one crop, one furrow direction and one texture phase and shows no seam.
- Cover vertices are lifted along the ray through the point, never along a tile normal: two facets
  agree on their shared edge, so both tiles land on the identical vertex. Lifting along each tile's
  own normal left a step of about 0.22 px at every seam.
- A wood is the exception to the cutting: crowns are never clipped to their tile, so the treeline
  spills past the hex border and the silhouette stays organic. A house is the same: it stands
  wherever its plot centre lands and keeps its full footprint.
- A village takes its wall tone per zone and its plot grid from the zone frame, so a settlement
  spanning several tiles is one street layout in one material rather than a tile's worth each.

## Testing
`tests/fixtures.rs` compares each ported builder with `fixtures/scenery/*.json`: vertex counts,
the exact `sha256` of the little-endian `f32` bytes, and the first 64 floats for readable diffs.
Farmland, the wood and the villages also have their zone, parcel, fence, tile, crown, house and
wing counts pinned, and their constants are checked against `fixtures/constants.json`. The polygon kit and the zone
machinery carry unit tests on hand-made shapes, where the expected answer can be read off by hand.

## Non-goals
GPU resources, materials and shaders (`cl-render`), gameplay.
