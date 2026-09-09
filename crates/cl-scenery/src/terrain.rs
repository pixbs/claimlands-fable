//! The planet's surface, prototype section 4 (`buildMesh`): one non-indexed triangle fan per tile
//! with atlas UVs, a cliff wedge wherever a tile stands above the tile across an edge, the surf
//! band beyond it, and the debug edge ribbons.
//!
//! Non-indexed keeps every tile's UVs and colours independent and makes triangle index → tile id a
//! flat array lookup.

use cl_hexsphere::{Frames, HexSphere};
use cl_model::world::{RADIUS, UV_INSET};
use cl_model::{MeshData, TileId, WorldSnapshot};
use cl_noise::hash2;
use cl_noise::vec::{V3, add, dot, len, mul, norm, sub};
use cl_pixelart::{Atlas, BEACH_PX, CLIFF_H, CLIFF_W, FOAM_PX, FOAM_W};

/// Half the width of the debug edge ribbon, in world pixels. A GL line is one *device* pixel and so
/// changes apparent width with zoom; a ribbon inset by half a world pixel is exactly one world
/// pixel of seam at every zoom, because both tiles draw their half.
pub const EDGE_HALF_PX: f64 = 0.5;
/// How far the edge ribbon floats above the facet, as a fraction of [`RADIUS`].
pub const EDGE_LIFT: f64 = 0.0010;
/// How far the surf floats above the water facet, so it never fights its plane.
pub const FOAM_LIFT: f64 = 1.0015;
/// Where the surf sits inside its strip, in `v`: the sheet is inset a little at both ends.
pub const FOAM_V: [f64; 2] = [0.98, 0.02];

/// The four meshes of the planet's surface plus the lookups picking and tinting need.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Terrain {
    /// The ground: one fan per tile, atlas UVs, white vertex colours, flat facet normals, and one
    /// [`TileId`] per triangle in `mesh.face_tile`.
    pub mesh: MeshData,
    /// The cliff wedges, with one [`TileId`] per triangle in `walls.face_tile`.
    pub walls: MeshData,
    /// The surf band beyond the wedges. No colours: the sheet carries them.
    pub foam: MeshData,
    /// Tile border ribbons for the debug toggle. Positions only, drawn unlit.
    pub edges: MeshData,
    /// First ground vertex of each tile, indexed by tile id: where the territory tint is written.
    pub vertex_start: Vec<usize>,
    /// Ground vertices of each tile, indexed by tile id (`sides * 3`).
    pub vertex_count: Vec<usize>,
}

impl Terrain {
    /// Tile owning each ground triangle: the prototype's `faceTile`, which turns a raycast hit into
    /// a tile id.
    pub fn face_tile(&self) -> &[TileId] {
        &self.mesh.face_tile
    }

    /// Tile owning each wall triangle: the prototype's `wallTile`. A cliff resolves to the tile
    /// above it.
    pub fn wall_tile(&self) -> &[TileId] {
        &self.walls.face_tile
    }
}

/// The prototype's `buildMesh`. `frames` and `atlas` must both come from `snapshot`.
///
/// # Panics
/// If `snapshot`, `frames`, `atlas` and `sphere` disagree on the tile count.
pub fn build_terrain(
    sphere: &HexSphere,
    frames: &Frames,
    snapshot: &WorldSnapshot,
    atlas: &Atlas,
) -> Terrain {
    let count = sphere.len();
    assert_eq!(snapshot.tiles.len(), count, "one tile state per tile");
    assert_eq!(frames.tiles.len(), count, "one frame per tile");
    assert_eq!(atlas.cells.len(), count, "one atlas cell per tile");
    let px = frames.px;
    let level: Vec<i32> = snapshot.levels();

    let mut out = Terrain {
        vertex_start: Vec::with_capacity(count),
        vertex_count: Vec::with_capacity(count),
        ..Terrain::default()
    };
    let cols = f64::from(atlas.cols);
    let rows = f64::from(atlas.rows);
    let mut vcount = 0;

    for (t, frame) in sphere.tiles.iter().zip(&frames.tiles) {
        let rad = frame.radius;
        let mid = frame.mid;
        let nn = frame.normal;
        let r_circum = frame.unit_circum;
        let sides = t.sides();
        // No per-tile tint: a uniform shift per hex would show as a brightness step at every border
        // and undo the seamless ground. The attribute is kept solely for the territory highlight.
        let tint = [1.0f32; 3];
        out.vertex_start.push(vcount);
        out.vertex_count.push(sides * 3);
        vcount += sides * 3;

        let [cell_col, cell_row] = atlas.cells[t.id.index()];
        let cell_col = f64::from(cell_col);
        let cell_row = f64::from(cell_row);
        let uv_of = |p: V3| -> [f64; 2] {
            let d = sub(p, t.center);
            let x = dot(d, t.e1) / r_circum;
            let y = dot(d, t.e2) / r_circum;
            [
                (cell_col + 0.5 + x * 0.5 * UV_INSET) / cols,
                (rows - 1.0 - cell_row + 0.5 + y * 0.5 * UV_INSET) / rows,
            ]
        };
        let uv_c = [
            (cell_col + 0.5) / cols,
            (rows - 1.0 - cell_row + 0.5) / rows,
        ];

        for k in 0..sides {
            let ca = t.corners[k];
            let cb = t.corners[(k + 1) % sides];
            let a = mul(ca, rad);
            let b = mul(cb, rad);
            let ua = uv_of(ca);
            let ub = uv_of(cb);
            for p in [mid, a, b] {
                out.mesh.push_vertex(p, nn);
            }
            for uv in [uv_c, ua, ub] {
                out.mesh.uvs.extend([uv[0] as f32, uv[1] as f32]);
            }
            for _ in 0..3 {
                out.mesh.colors.extend(tint);
            }
            out.mesh.face_tile.push(t.id);
        }

        // The ribbon is scaled about the centroid using each edge's own apothem: one scale for the
        // whole tile is exact only for a regular polygon, and tiles near the pentagons are not.
        let gw = EDGE_HALF_PX * px;
        let goff = mul(nn, EDGE_LIFT * RADIUS);
        for k in 0..sides {
            let a = add(mul(t.corners[k], rad), goff);
            let b = add(mul(t.corners[(k + 1) % sides], rad), goff);
            let gd = norm(sub(b, a));
            let gv = sub(mid, a);
            // Perpendicular distance, not the distance to the midpoint.
            let gap = len(sub(gv, mul(gd, dot(gv, gd))));
            let gs = (gap - gw) / gap;
            let ai = add(mid, add(mul(sub(a, mid), gs), mul(goff, 1.0 - gs)));
            let bi = add(mid, add(mul(sub(b, mid), gs), mul(goff, 1.0 - gs)));
            for p in [a, b, bi, a, bi, ai] {
                out.edges
                    .positions
                    .extend([p[0] as f32, p[1] as f32, p[2] as f32]);
            }
        }
    }

    coast(sphere, frames, &level, px, &mut out);
    out
}

/// The displacement of corner `k` of `tile`, or `None` where the corner is not on a level change.
///
/// It is a property of the *corner*, derived from the three tiles meeting there, not of the tile
/// asking for it — so every tile touching that corner pushes it to the identical point and the
/// coastline cannot crack.
fn corner_push(
    sphere: &HexSphere,
    level: &[i32],
    tile: &cl_hexsphere::Tile,
    k: usize,
) -> Option<V3> {
    let trio = tile.corner_tiles[k];
    let mut lo = [0.0, 0.0, 0.0];
    let mut hi = [0.0, 0.0, 0.0];
    let mut nl = 0.0;
    let mut nh = 0.0;
    let mut min = i32::MAX;
    for id in trio {
        min = min.min(level[id.index()]);
    }
    for id in trio {
        let c = sphere.tile(id).center;
        if level[id.index()] == min {
            lo = add(lo, c);
            nl += 1.0;
        } else {
            hi = add(hi, c);
            nh += 1.0;
        }
    }
    if nh == 0.0 {
        return None;
    }
    let c = tile.corners[k];
    let dir = sub(mul(lo, 1.0 / nl), mul(hi, 1.0 / nh));
    let tang = sub(dir, mul(c, dot(dir, c)));
    let l = len(tang);
    if l < 1e-9 {
        None
    } else {
        Some(mul(tang, 1.0 / l))
    }
}

/// A corner moved `amt` along its push, back onto the unit sphere.
fn shift(c: V3, d: Option<V3>, amt: f64) -> V3 {
    match d {
        Some(d) => norm(add(c, mul(d, amt))),
        None => c,
    }
}

/// The coast: instead of a square cliff the drop is a wedge, its top edge on the land tile's edge
/// and its bottom edge `BEACH_PX` out over the water, and the surf band carries on from there into
/// open water. The two land tiles either side of a coastal corner both build a ramp reaching it, so
/// the wedges join without end caps.
fn coast(sphere: &HexSphere, frames: &Frames, level: &[i32], px: f64, out: &mut Terrain) {
    let beach = BEACH_PX * px;
    let surf = (BEACH_PX + FOAM_PX) * px;
    let tint = [1.0f32; 3];

    for (t, frame) in sphere.tiles.iter().zip(&frames.tiles) {
        let rad = frame.radius;
        let mid = frame.mid;
        let sides = t.sides();
        for k in 0..sides {
            let other = t.edge_neighbors[k];
            if level[other.index()] >= level[t.id.index()] {
                continue;
            }
            let k2 = (k + 1) % sides;
            let ca = t.corners[k];
            let cb = t.corners[k2];
            let r_b = frames.tiles[other.index()].radius;
            let d_a = corner_push(sphere, level, t, k);
            let d_b = corner_push(sphere, level, t, k2);

            let at = mul(ca, rad);
            let bt = mul(cb, rad);
            let ab = mul(shift(ca, d_a, beach), r_b);
            let bb = mul(shift(cb, d_b, beach), r_b);

            // Newell over the four corners: the wedge face is no longer planar.
            let mut wn = [0.0, 0.0, 0.0];
            let ring = [at, ab, bb, bt];
            for q in 0..4 {
                let p = ring[q];
                let r = ring[(q + 1) % 4];
                wn[0] += (p[1] - r[1]) * (p[2] + r[2]);
                wn[1] += (p[2] - r[2]) * (p[0] + r[0]);
                wn[2] += (p[0] - r[0]) * (p[1] + r[1]);
            }
            let mut wn = norm(wn);
            if dot(wn, sub(mul(add(at, bt), 0.5), mid)) < 0.0 {
                wn = mul(wn, -1.0);
            }

            // Phase so segments along a coast do not align.
            let u0 = hash2(f64::from(t.id.0) * 0.53 + k as f64 * 2.1, k as f64 * 1.7);
            let u1 = u0 + len(sub(bt, at)) / (px * f64::from(CLIFF_W));
            // Slant length, not height: that keeps the wall texels square.
            let v_top = len(sub(ab, at)) / (px * f64::from(CLIFF_H));

            for p in [at, ab, bb, at, bb, bt] {
                out.walls.push_vertex(p, wn);
            }
            for uv in [
                [u0, v_top],
                [u0, 0.0],
                [u1, 0.0],
                [u0, v_top],
                [u1, 0.0],
                [u1, v_top],
            ] {
                out.walls.uvs.extend([uv[0] as f32, uv[1] as f32]);
            }
            for _ in 0..6 {
                out.walls.colors.extend(tint);
            }
            out.walls.face_tile.extend([t.id, t.id]);

            // Surf, laid flat on the water just beyond the wedge.
            let r_f = r_b * FOAM_LIFT;
            let ai = mul(shift(ca, d_a, beach), r_f);
            let bi = mul(shift(cb, d_b, beach), r_f);
            let ao = mul(shift(ca, d_a, surf), r_f);
            let bo = mul(shift(cb, d_b, surf), r_f);
            let fu1 = u0 + len(sub(bi, ai)) / (px * f64::from(FOAM_W));
            let normal = frames.tiles[other.index()].normal;
            for p in [ai, ao, bo, ai, bo, bi] {
                out.foam.push_vertex(p, normal);
            }
            for uv in [
                [u0, FOAM_V[0]],
                [u0, FOAM_V[1]],
                [fu1, FOAM_V[1]],
                [u0, FOAM_V[0]],
                [fu1, FOAM_V[1]],
                [fu1, FOAM_V[0]],
            ] {
                out.foam.uvs.extend([uv[0] as f32, uv[1] as f32]);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_hexsphere::compute_tile_frames;
    use cl_model::{Terrain as TerrainKind, TileState};
    use cl_pixelart::build_terrain_atlas;

    use super::*;

    fn world() -> (HexSphere, WorldSnapshot) {
        let sphere = HexSphere::build(3);
        let tiles: Vec<TileState> = sphere
            .tiles
            .iter()
            .map(|t| TileState {
                terrain: if t.center[2] > 0.0 {
                    TerrainKind::Land
                } else {
                    TerrainKind::Sea
                },
                ..TileState::default()
            })
            .collect();
        (
            sphere,
            WorldSnapshot {
                frequency: 3,
                seed: 4242,
                tiles,
            },
        )
    }

    #[test]
    fn every_mesh_is_consistent_and_indexed_by_tile() {
        let (sphere, snapshot) = world();
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let atlas = build_terrain_atlas(&sphere, &frames, &snapshot, f64::from(snapshot.seed));
        let terrain = build_terrain(&sphere, &frames, &snapshot, &atlas);

        for mesh in [&terrain.mesh, &terrain.walls, &terrain.foam, &terrain.edges] {
            assert!(mesh.validate().is_ok(), "{:?}", mesh.validate());
        }
        let fans: usize = sphere.tiles.iter().map(|t| t.sides()).sum();
        assert_eq!(terrain.mesh.triangle_count(), fans);
        assert_eq!(terrain.face_tile().len(), fans);
        assert_eq!(terrain.edges.triangle_count(), fans * 2);
        assert_eq!(terrain.wall_tile().len(), terrain.walls.triangle_count());
        assert_eq!(
            terrain.foam.triangle_count(),
            terrain.walls.triangle_count()
        );
        assert!(terrain.walls.triangle_count() > 0, "the world has a coast");

        // vertex_start / vertex_count address exactly this tile's fan.
        for t in &sphere.tiles {
            let start = terrain.vertex_start[t.id.index()];
            assert_eq!(terrain.vertex_count[t.id.index()], t.sides() * 3);
            for k in 0..t.sides() {
                assert_eq!(terrain.face_tile()[start / 3 + k], t.id);
            }
        }
        assert_eq!(
            terrain.vertex_start.last().unwrap() + terrain.vertex_count.last().unwrap(),
            terrain.mesh.vertex_count()
        );
    }

    /// The wedge bottom is a property of the corner, so the two land tiles meeting at a coastal
    /// corner must place it at the identical point — otherwise the coastline cracks.
    #[test]
    fn a_shared_coastal_corner_is_pushed_to_one_point() {
        let (sphere, snapshot) = world();
        let level = snapshot.levels();
        let mut checked = 0;
        for t in &sphere.tiles {
            for k in 0..t.sides() {
                let Some(push) = corner_push(&sphere, &level, t, k) else {
                    continue;
                };
                let corner = t.corners[k];
                for &id in &t.corner_tiles[k] {
                    if id == t.id {
                        continue;
                    }
                    let other = sphere.tile(id);
                    let Some(j) =
                        (0..other.sides()).find(|&j| len(sub(other.corners[j], corner)) < 1e-12)
                    else {
                        continue;
                    };
                    let theirs = corner_push(&sphere, &level, other, j).expect("same corner");
                    assert_eq!(theirs, push, "corner {k} of {} vs {id}", t.id);
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "the world has coastal corners");
    }
}
