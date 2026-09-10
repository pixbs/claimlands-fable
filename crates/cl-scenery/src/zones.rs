//! Cover zones and their tangent frames, prototype `coverZones` and `zoneFrame`.
//!
//! A zone is a connected run of tiles carrying the same cover, small enough that one tangent plane
//! describes it without stretching. It is the unit every cover renderer lays out as a single piece
//! of scenery before cutting it along the hex borders, so the scenery merges across tile seams.

use cl_hexsphere::HexSphere;
use cl_model::{Cover, TileId, WorldSnapshot};
use cl_noise::hash3i;
use cl_noise::js;
use cl_noise::vec::{V3, add, cross, dot, len, mul, norm, sub};

/// The zones of one cover over the whole sphere.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Zoning {
    /// Tiles of each zone, in the order the flood fill reached them.
    pub zones: Vec<Vec<TileId>>,
    /// Zone index of each tile, `-1` for a tile outside every zone.
    pub zone_of: Vec<i32>,
    /// Tiles carrying the cover, over all zones.
    pub total: usize,
}

/// The prototype's `coverZones`: land tiles carrying `kind`, flood-filled into groups that stay
/// within `span` radians of the tile the group started from.
pub(crate) fn cover_zones(
    sphere: &HexSphere,
    snapshot: &WorldSnapshot,
    kind: Cover,
    span: f64,
) -> Zoning {
    let tiles = &sphere.tiles;
    let mut list = Vec::new();
    let mut mark = vec![false; tiles.len()];
    for t in tiles {
        let state = &snapshot.tiles[t.id.index()];
        if state.cover == kind && state.terrain.level() >= 0 {
            list.push(t.id);
            mark[t.id.index()] = true;
        }
    }
    let mut zone_of = vec![-1i32; tiles.len()];
    let mut zones: Vec<Vec<TileId>> = Vec::new();
    let cap = js::cos(span);
    for &s in &list {
        if zone_of[s.index()] >= 0 {
            continue;
        }
        let z = zones.len() as i32;
        let mut group = Vec::new();
        let mut queue = vec![s];
        zone_of[s.index()] = z;
        let mut head = 0;
        while head < queue.len() {
            let t = queue[head];
            head += 1;
            group.push(t);
            for &j in &tiles[t.index()].neighbors {
                if !mark[j.index()] || zone_of[j.index()] >= 0 {
                    continue;
                }
                // The cap is measured from the tile the zone started at, so a zone never grows
                // wider than one tangent plane can describe.
                if dot(tiles[j.index()].center, tiles[s.index()].center) < cap {
                    continue;
                }
                zone_of[j.index()] = z;
                queue.push(j);
            }
        }
        zones.push(group);
    }
    Zoning {
        zones,
        zone_of,
        total: list.len(),
    }
}

/// Tangent frame of one zone, spun by an integer hash so no two zones share a layout grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ZoneFrame {
    /// Unit direction of the zone centre; the tangent plane touches the sphere here.
    pub c: V3,
    /// First tangent axis, spun by the zone's hash.
    pub e1: V3,
    /// `c × e1`; `(e1, e2, c)` is right-handed.
    pub e2: V3,
}

impl ZoneFrame {
    /// The prototype's `zoneFrame(zone, salt)`.
    pub fn new(sphere: &HexSphere, zone: &[TileId], salt: i32) -> Self {
        let mut c = [0.0, 0.0, 0.0];
        for &id in zone {
            c = add(c, sphere.tiles[id.index()].center);
        }
        let c = norm(c);
        let r = if c[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let a1 = norm(sub(r, mul(c, dot(r, c))));
        let a2 = cross(c, a1);
        let spin = hash3i(
            zone[0].0 as i32,
            i32::try_from(zone.len()).expect("zone fits in i32"),
            salt,
        ) * std::f64::consts::PI
            * 2.0;
        let e1 = add(mul(a1, js::cos(spin)), mul(a2, js::sin(spin)));
        let e2 = cross(c, e1);
        ZoneFrame { c, e1, e2 }
    }

    /// The prototype's `to2e`: equal-distance projection into the tangent plane. The distance from
    /// the zone centre is carried through unchanged, so a cell of a regular grid keeps its size
    /// radially and loses only `sin(p)/p` across — 6 % at the far edge of a 0.6 rad zone, against
    /// 22 % for the gnomonic projection the parcel clipper needs.
    pub fn to2e(&self, p: V3) -> [f64; 2] {
        let d = (-1.0f64).max(1.0f64.min(dot(p, self.c)));
        let rho = libm::acos(d);
        let t = sub(p, mul(self.c, d));
        let l = len(t);
        if l < 1e-12 {
            return [0.0, 0.0];
        }
        [dot(t, self.e1) / l * rho, dot(t, self.e2) / l * rho]
    }

    /// Inverse of [`ZoneFrame::to2e`], the prototype's `to3e`.
    pub fn to3e(&self, q: [f64; 2]) -> V3 {
        let rho = js::hypot2(q[0], q[1]);
        if rho < 1e-12 {
            return self.c;
        }
        let u = mul(add(mul(self.e1, q[0]), mul(self.e2, q[1])), 1.0 / rho);
        add(mul(self.c, js::cos(rho)), mul(u, js::sin(rho)))
    }
}

/// Signed area of a 2D polygon, the prototype's `polyArea`. Negative means clockwise.
pub(crate) fn poly_area(p: &[[f64; 2]]) -> f64 {
    let mut a = 0.0;
    for i in 0..p.len() {
        let q = p[(i + 1) % p.len()];
        a += p[i][0] * q[1] - q[0] * p[i][1];
    }
    a / 2.0
}

/// Is `q` inside the convex counter-clockwise polygon `h`? The prototype's `inHull`.
pub(crate) fn in_hull(h: &[[f64; 2]], q: [f64; 2]) -> bool {
    for k in 0..h.len() {
        let a = h[k];
        let b = h[(k + 1) % h.len()];
        if (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]) < -1e-12 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cl_model::{Terrain, TileState};
    use cl_noise::vec::len;

    fn snapshot(frequency: u8, forest: &[usize]) -> WorldSnapshot {
        let count = cl_model::world::tile_count(frequency);
        let mut tiles = vec![
            TileState {
                terrain: Terrain::Land,
                ..TileState::default()
            };
            count
        ];
        for &i in forest {
            tiles[i].cover = Cover::Forest;
        }
        WorldSnapshot {
            frequency,
            seed: 1,
            tiles,
        }
    }

    #[test]
    fn a_zone_is_one_connected_run_of_the_cover() {
        let sphere = HexSphere::build(4);
        // A tile and one of its neighbours join; a tile on the far side of the planet does not.
        let a = 0usize;
        let b = sphere.tiles[a].neighbors[0].index();
        let far = sphere
            .tiles
            .iter()
            .position(|t| dot(t.center, sphere.tiles[a].center) < -0.9)
            .expect("an antipodal tile");
        let z = cover_zones(&sphere, &snapshot(4, &[a, b, far]), Cover::Forest, 0.6);
        assert_eq!(z.total, 3);
        assert_eq!(z.zones.len(), 2);
        assert_eq!(z.zones[0].len(), 2);
        assert_eq!(z.zone_of[far], 1);
        assert_eq!(z.zone_of[sphere.tiles[a].neighbors[1].index()], -1);
    }

    #[test]
    fn sea_tiles_never_join_a_zone() {
        let sphere = HexSphere::build(4);
        let mut snap = snapshot(4, &[0, sphere.tiles[0].neighbors[0].index()]);
        snap.tiles[0].terrain = Terrain::Sea;
        let z = cover_zones(&sphere, &snap, Cover::Forest, 0.6);
        assert_eq!(z.total, 1);
        assert_eq!(z.zone_of[0], -1);
    }

    #[test]
    fn the_equal_distance_projection_round_trips_and_keeps_distances() {
        let sphere = HexSphere::build(4);
        let zone = vec![TileId(0)];
        let frame = ZoneFrame::new(&sphere, &zone, 29);
        for &corner in &sphere.tiles[0].corners {
            let q = frame.to2e(corner);
            let back = frame.to3e(q);
            assert!(len(sub(back, corner)) < 1e-12, "round trip");
            // Equal-distance: the planar radius is the angle from the zone centre.
            let rho = libm::acos(dot(corner, frame.c));
            assert!((js::hypot2(q[0], q[1]) - rho).abs() < 1e-12, "radius kept");
        }
        assert_eq!(frame.to2e(frame.c), [0.0, 0.0]);
        assert_eq!(frame.to3e([0.0, 0.0]), frame.c);
    }

    #[test]
    fn hull_tests_agree_with_the_winding() {
        let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert_eq!(poly_area(&square), 1.0);
        assert!(in_hull(&square, [0.5, 0.5]));
        assert!(!in_hull(&square, [1.5, 0.5]));
        let mut clockwise = square;
        clockwise.reverse();
        assert_eq!(poly_area(&clockwise), -1.0);
        assert!(!in_hull(&clockwise, [0.5, 0.5]));
    }
}
