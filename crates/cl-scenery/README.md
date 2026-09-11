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
| `build_clouds(&sphere, &frames, px)` -> `Clouds` (the shared shell plus a `Deck` per cloud layer: scale, tone, draw order), `hole_rest()`, `hole_at(camera_distance)` | `buildClouds`, the see-through half of `stepSky` | ported |
| `build_terrain(&sphere, &frames, &snapshot, &atlas)` -> `Terrain` (fans, cliff wedges, surf, edge ribbons, `face_tile`, `wall_tile`, `vertex_start`/`vertex_count`) | `buildMesh` | ported |
| `build_fields(&sphere, &frames, &snapshot, px)` -> `Option<Fields>` (surface, posts, zone/parcel/fence counts) | `buildFields` | ported |
| `build_forest(&sphere, &frames, &snapshot, px, seed)` -> `Option<Forest>` (crowns and understory discs in one mesh, plus zone, tile and crown counts) | `buildForest` | ported |
| `cover_zones`, `zone_frame`, `ZoneFrame::{to2, to3, to2e, to3e}`, `ring_normal` | `coverZones`, `zoneFrame`, `ringNormal` | ported |
| `poly`: `clip_half`, `clip_to_hull`, `trim_convex`, `dedupe`, `mitre_offset`, `parcel_split`, `poly_area`, `poly_thickness`, `hull_at` | the 2D convex polygon kit | ported |
| `build_houses(&sphere, &frames, &snapshot, px, seed)` -> `Option<Houses>` (every house and wing in one mesh, plus zone, tile, house and wing counts) | `buildHouses` | ported |
| `build_border`, `hover_ring` | `rebuildBorder`, `setRing` | issue M1 |
| `build_space(w, h)` -> `Space` (vignette, star quads, `Star` list), `step_stars(&mut colors, &list, t)` | `buildSpace`, `stepStars` | ported |
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
- The cloud decks share one shell and differ only by a uniform scale, so they stay exactly
  concentric however the shell is built and the GPU holds the geometry once. Deck `k` stands
  `k * CLOUD_LIFT_PX` above the first.
- The see-through hole rests shut (`HOLE_REST_OPEN` is 1), so a deck costs its fragment nothing
  until something opens the cap. Its cosines go through `cl_noise::js::cos`, not `libm`, because
  the prototype's values come from V8's `Math.cos`.
- `hole_at` tracks the camera's distance, not a clock: the opening widens and deepens together as
  the camera closes in. Its window (6.0 down to 5.0) sits almost at full zoom-out on purpose — the
  camera opens at 3.3, so the hole is already open on the first frame. A deck with the hole left
  shut reads as a solid blanket, which is what the prototype shows only at the top of the range.
- The backdrop is the one builder in clip space rather than world space, and the one whose input is
  the render target rather than the world: its stars are quads snapped to the render-pixel lattice,
  so a new target size means a new star field. The prototype's plane is indexed and `MeshData` is
  not, so the vignette's 15 × 15 grid is expanded into the same plain triangle list as everything
  else; the fixture is compared through the plane's index.

## Testing
`tests/fixtures.rs` compares each ported builder with `fixtures/scenery/*.json`: vertex counts,
the exact `sha256` of the little-endian `f32` bytes, and the first 64 floats for readable diffs.
Farmland, the wood and the villages also have their zone, parcel, fence, tile, crown, house and
wing counts pinned, and their constants are checked against `fixtures/constants.json`. The cloud
stack is pinned against `fixtures/scenery/clouds-n8.json` — shell radius, tile count, and per deck
the scale to the bit, the draw order, the tint, `alphaTest`, the sides and the resting hole angles.
`uFocus` is not compared: the harness's `Vector3` shim drops its arguments, so the fixture records
`(0,0,0)` where the prototype passes `(0,0,1)`, and pinning it would pin the shim. The backdrop is
compared float for float at both fixture sizes — vignette, star quads, the whole `Star` list, and the
star colours after two flicker steps. The polygon kit and the zone machinery carry unit tests on
hand-made shapes, where the expected answer can be read off by hand.

## Non-goals
GPU resources, materials and shaders (`cl-render`), gameplay.
