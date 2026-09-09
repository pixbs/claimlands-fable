# cl-hexsphere

## Purpose
The planet's tiling: a Goldberg polyhedron, the dual of a frequency-`n` geodesic icosahedron, giving
`10n² + 2` tiles of which exactly twelve are pentagons. Every tile knows its corners (CCW seen from
outside), a stable tangent frame `e1/e2`, which tile lies across each edge, which three tiles meet at
each corner, and its neighbours in insertion order. The per-tile facet frames (radius, centroid,
flat normal, apothem, texel size) are computed here too because both the ground atlas and the mesh
builders need them. Ported one-to-one from prototype sections 1 and 4 (`hex-planet.html` lines
183–276, 652–697, 2047–2062, 2727–2734).

## Public API
| Item | Prototype | Notes |
|---|---|---|
| `icosahedron()` | `icosahedron` | 12 unit vertices, 20 faces |
| `geodesic(n)` | `geodesic` | vertices deduplicated by the exact `toFixed(6)` key, so ids match the prototype |
| `HexSphere::build(n)` | `buildHexSphere` | tiles by geodesic vertex order; implements `cl_model::Board` |
| `Tile` | tile object | `center`, `corners`, `e1`, `e2`, `edge_neighbors`, `corner_tiles`, `neighbors` |
| `compute_tile_frames(&sphere, &levels)` | `computeTileFrames` | `Frames { px, step, tiles }`; `px` is one texture pixel in world units |
| `facet_plane`, `texel_dir` | `facetPlane`, `texelDir` | the atlas' texel → direction inverse |
| `tiles_around(&sphere)` | `tilesAround` | tiles a pawn would cross to circle the planet |

## Invariants
- `tiles.len() == 10n² + 2`; exactly 12 tiles have 5 corners, the rest 6.
- Tile ids and neighbour order are pinned by `fixtures/hexsphere/n{2..12}.json`; changing the
  build order is a breaking change for every level string ever shared.
- `edge_neighbors[k]` is the tile across the edge from corner `k` to corner `k+1`; `corner_tiles[k]`
  are the three tiles meeting at corner `k` and always include the tile itself.
- All geometry is `f64`; nothing here allocates per texel.

## Testing
`tests/fixtures.rs` compares every tile of every frequency with the prototype: counts, pentagon ids,
neighbour lists (exact), coordinates (bit-exact for `n ≤ 6`, quantised hash for larger `n`), tile
frames and texel directions for the fixture worlds.

## Non-goals
Terrain, cover, meshes, textures, picking.
