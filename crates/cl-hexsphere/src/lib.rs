//! Goldberg hex sphere: the dual of a geodesic icosahedron. A frequency-`n` sphere always yields
//! `10n² + 2` tiles, exactly twelve of them pentagons. Pure geometry in `f64`, ported one-to-one
//! from prototype section 1 so that tile ids, corner order and neighbour order are identical.
#![forbid(unsafe_code)]

mod frames;
mod geodesic;

use cl_model::{Board, TileId};
use cl_noise::vec::{V3, add, cross, dot, mul, norm, sub};

pub use frames::{
    FacetPlane, Frames, TileFrame, compute_tile_frames, facet_plane, texel_dir, tiles_around,
};
pub use geodesic::{Geodesic, geodesic, icosahedron};

/// One tile of the sphere: a hexagon or pentagon on the unit sphere.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// Index, equal to the geodesic vertex index.
    pub id: TileId,
    /// Unit direction of the tile centre.
    pub center: V3,
    /// Unit corners, counter-clockwise seen from outside; `corners[0]` fixes the frame.
    pub corners: Vec<V3>,
    /// Tangent axis aimed at the first corner, so the tile's texture keeps a stable orientation.
    pub e1: V3,
    /// `center × e1`; `(e1, e2, center)` is right-handed.
    pub e2: V3,
    /// Tile across the edge from corner `k` to corner `k+1`.
    pub edge_neighbors: Vec<TileId>,
    /// The three tiles meeting at corner `k` (this tile included), in geodesic triangle order.
    pub corner_tiles: Vec<[TileId; 3]>,
    /// Adjacent tiles in first-seen order over the geodesic triangles.
    pub neighbors: Vec<TileId>,
}

impl Tile {
    /// 5 or 6.
    pub fn sides(&self) -> usize {
        self.corners.len()
    }

    /// One of the twelve pentagons.
    pub fn is_pentagon(&self) -> bool {
        self.corners.len() == 5
    }
}

/// The complete tiling for one frequency.
#[derive(Debug, Clone, PartialEq)]
pub struct HexSphere {
    /// Subdivision frequency `n`.
    pub frequency: u8,
    /// Tiles indexed by id.
    pub tiles: Vec<Tile>,
}

impl HexSphere {
    /// The prototype's `buildHexSphere(n)`.
    ///
    /// # Panics
    /// If `n` is outside `2..=12`.
    pub fn build(n: u8) -> Self {
        assert!(
            cl_model::world::frequency_is_valid(n),
            "frequency {n} outside 2..=12"
        );
        let Geodesic { verts, tris } = geodesic(n);
        let count = verts.len();
        let mut around: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut nbr: Vec<Vec<TileId>> = vec![Vec::new(); count];
        for (ti, t) in tris.iter().enumerate() {
            for k in 0..3 {
                let v = t[k] as usize;
                around[v].push(ti);
                for other in [t[(k + 1) % 3], t[(k + 2) % 3]] {
                    let id = TileId(other);
                    if !nbr[v].contains(&id) {
                        nbr[v].push(id);
                    }
                }
            }
        }
        let centroid: Vec<V3> = tris
            .iter()
            .map(|t| {
                norm(add(
                    add(verts[t[0] as usize], verts[t[1] as usize]),
                    verts[t[2] as usize],
                ))
            })
            .collect();

        let tiles = verts
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                // Tangent frame with e1 aimed at the first corner, then corners sorted CCW by angle.
                let first = centroid[around[i][0]];
                let e1 = norm(sub(first, mul(c, dot(first, c))));
                let e2 = cross(c, e1);
                let mut fan: Vec<(usize, V3, f64)> = around[i]
                    .iter()
                    .map(|&ti| {
                        let p = centroid[ti];
                        (ti, p, libm::atan2(dot(p, e2), dot(p, e1)))
                    })
                    .collect();
                fan.sort_by(|x, y| x.2.partial_cmp(&y.2).expect("angles are finite"));
                let corners: Vec<V3> = fan.iter().map(|o| o.1).collect();
                let sides = fan.len();
                // Across edge k lies the vertex the two geodesic triangles at corners k and k+1 share.
                let edge_neighbors: Vec<TileId> = (0..sides)
                    .map(|k| {
                        let a = tris[fan[k].0];
                        let b = tris[fan[(k + 1) % sides].0];
                        a.iter()
                            .copied()
                            .find(|&v| v as usize != i && b.contains(&v))
                            .map(TileId)
                            .expect("a closed mesh has a tile across every edge")
                    })
                    .collect();
                let corner_tiles: Vec<[TileId; 3]> =
                    fan.iter().map(|o| tris[o.0].map(TileId)).collect();
                Tile {
                    id: TileId(i as u32),
                    center: c,
                    corners,
                    e1,
                    e2,
                    edge_neighbors,
                    corner_tiles,
                    neighbors: nbr[i].clone(),
                }
            })
            .collect();
        HexSphere {
            frequency: n,
            tiles,
        }
    }

    /// Number of tiles.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Never true for a valid frequency.
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Ids of the twelve pentagons.
    pub fn pentagons(&self) -> Vec<TileId> {
        self.tiles
            .iter()
            .filter(|t| t.is_pentagon())
            .map(|t| t.id)
            .collect()
    }
}

impl Board for HexSphere {
    fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    fn neighbors(&self, id: TileId) -> &[TileId] {
        &self.tiles[id.index()].neighbors
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_pentagons() {
        for n in 2..=5u8 {
            let s = HexSphere::build(n);
            assert_eq!(s.len(), cl_model::world::tile_count(n));
            assert_eq!(s.pentagons().len(), 12);
            for t in &s.tiles {
                assert_eq!(t.neighbors.len(), t.sides());
                assert_eq!(t.edge_neighbors.len(), t.sides());
                assert!(t.corner_tiles.iter().all(|c| c.contains(&t.id)));
            }
        }
    }
}
