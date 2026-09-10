//! What every cover renderer shares: zones, their tangent frames, and two small 3D helpers.
//!
//! A *zone* is a connected run of tiles carrying the same cover, small enough that one tangent plane
//! describes it without stretching. A zone is exactly the thing that visually merges: everything
//! inside one is laid out as a single piece of scenery and only afterwards cut up along the hex
//! borders, which is why a field straddling a tile border shows no seam.

use cl_hexsphere::HexSphere;
use cl_model::{Cover, TileId, WorldSnapshot};
use cl_noise::hash3i;
use cl_noise::js::{cos, hypot2, sin};
use cl_noise::vec::{V3, add, cross, dot, len, mul, norm, sub};

/// Connected runs of tiles carrying `kind`, none spanning more than `span` radians of sky.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Zones {
    /// Tiles of each zone, in the order the flood fill reached them.
    pub zones: Vec<Vec<TileId>>,
    /// Zone index per tile, or `-1` where the tile is not in one.
    pub zone_of: Vec<i32>,
    /// How many tiles carry `kind` altogether.
    pub total: usize,
}

/// The prototype's `coverZones`.
pub fn cover_zones(sphere: &HexSphere, snapshot: &WorldSnapshot, kind: Cover, span: f64) -> Zones {
    let count = sphere.len();
    let mut mark = vec![false; count];
    let mut list = Vec::new();
    for t in &sphere.tiles {
        let state = snapshot.tile(t.id);
        if state.cover == kind && state.terrain.is_land() {
            list.push(t.id);
            mark[t.id.index()] = true;
        }
    }
    let mut zone_of = vec![-1i32; count];
    let mut zones: Vec<Vec<TileId>> = Vec::new();
    let cap = cos(span);
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
            let id = queue[head];
            head += 1;
            group.push(id);
            for &j in &sphere.tile(id).neighbors {
                if !mark[j.index()] || zone_of[j.index()] >= 0 {
                    continue;
                }
                if dot(sphere.tile(j).center, sphere.tile(s).center) < cap {
                    continue;
                }
                zone_of[j.index()] = z;
                queue.push(j);
            }
        }
        zones.push(group);
    }
    Zones {
        zones,
        zone_of,
        total: list.len(),
    }
}

/// A zone's tangent frame, spun by an integer hash so no two zones on the planet share a layout
/// grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneFrame {
    /// Unit direction of the zone's centre.
    pub c: V3,
    /// First tangent axis.
    pub e1: V3,
    /// Second tangent axis; `(e1, e2, c)` is right-handed.
    pub e2: V3,
}

/// The prototype's `zoneFrame`.
pub fn zone_frame(sphere: &HexSphere, zone: &[TileId], salt: i32) -> ZoneFrame {
    let mut c = [0.0, 0.0, 0.0];
    for &id in zone {
        c = add(c, sphere.tile(id).center);
    }
    let c = norm(c);
    let reference = if c[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let a1 = norm(sub(reference, mul(c, dot(reference, c))));
    let a2 = cross(c, a1);
    let spin = hash3i(zone[0].0 as i32, zone.len() as i32, salt) * std::f64::consts::PI * 2.0;
    let e1 = add(mul(a1, cos(spin)), mul(a2, sin(spin)));
    let e2 = cross(c, e1);
    ZoneFrame { c, e1, e2 }
}

impl ZoneFrame {
    /// Gnomonic projection: rays from the planet's centre out to the tangent plane. A straight edge
    /// in 3D stays a straight edge here, so working in the plane and lifting the answer back is
    /// exact rather than an approximation.
    pub fn to2(&self, p: V3) -> [f64; 2] {
        let w = dot(p, self.c);
        [dot(p, self.e1) / w, dot(p, self.e2) / w]
    }

    /// Inverse of [`ZoneFrame::to2`].
    pub fn to3(&self, q: [f64; 2]) -> V3 {
        norm(add(self.c, add(mul(self.e1, q[0]), mul(self.e2, q[1]))))
    }

    /// Equal-distance variant, for anything laid out on a regular grid.
    ///
    /// Under gnomonic projection a cell a third of a radian off centre comes back up to 22 %
    /// smaller than it went in, which would leave a border asked to be one world pixel measuring
    /// 0.8 — and at roughly one world pixel per screen pixel that shows as an outline with holes in
    /// it. Here the distance from the zone centre is carried through unchanged, so a cell keeps its
    /// size radially and loses only `sin(p) / p` across, about 3 % at the far edge.
    pub fn to2e(&self, p: V3) -> [f64; 2] {
        let d = dot(p, self.c).clamp(-1.0, 1.0);
        let rho = libm::acos(d);
        let t = sub(p, mul(self.c, d));
        let l = len(t);
        if l < 1e-12 {
            return [0.0, 0.0];
        }
        [dot(t, self.e1) / l * rho, dot(t, self.e2) / l * rho]
    }

    /// Inverse of [`ZoneFrame::to2e`].
    pub fn to3e(&self, q: [f64; 2]) -> V3 {
        let rho = hypot2(q[0], q[1]);
        if rho < 1e-12 {
            return self.c;
        }
        let u = mul(add(mul(self.e1, q[0]), mul(self.e2, q[1])), 1.0 / rho);
        add(mul(self.c, cos(rho)), mul(u, sin(rho)))
    }
}

/// Newell normal of a ring of 3D points.
pub fn ring_normal(r: &[V3]) -> V3 {
    let mut n = [0.0, 0.0, 0.0];
    for i in 0..r.len() {
        let p = r[i];
        let q = r[(i + 1) % r.len()];
        n[0] += (p[1] - q[1]) * (p[2] + q[2]);
        n[1] += (p[2] - q[2]) * (p[0] + q[0]);
        n[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    norm(n)
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_model::{Terrain, TileState};

    use super::*;

    fn world(covers: &[(usize, Cover)]) -> (HexSphere, WorldSnapshot) {
        let sphere = HexSphere::build(3);
        let mut snapshot = WorldSnapshot {
            frequency: 3,
            seed: 7,
            tiles: vec![
                TileState {
                    terrain: Terrain::Land,
                    ..TileState::default()
                };
                sphere.len()
            ],
        };
        for &(i, c) in covers {
            snapshot.tiles[i].cover = c;
        }
        (sphere, snapshot)
    }

    #[test]
    fn zones_are_connected_runs_of_one_cover() {
        let (sphere, snapshot) = world(&[(0, Cover::Field)]);
        let z = cover_zones(&sphere, &snapshot, Cover::Field, 0.4);
        assert_eq!(z.total, 1);
        assert_eq!(z.zones.len(), 1);
        assert_eq!(z.zones[0], vec![TileId(0)]);
        assert_eq!(z.zone_of[0], 0);
        assert!(z.zone_of[1] < 0, "a tile with no field is in no zone");

        // A tile and one of its neighbours merge; a far tile does not.
        let near = sphere.tile(TileId(0)).neighbors[0];
        let far = (0..sphere.len())
            .map(|i| TileId(i as u32))
            .find(|&id| id != TileId(0) && !sphere.tile(TileId(0)).neighbors.contains(&id))
            .unwrap();
        let (sphere, snapshot) = world(&[
            (0, Cover::Field),
            (near.index(), Cover::Field),
            (far.index(), Cover::Field),
        ]);
        let z = cover_zones(&sphere, &snapshot, Cover::Field, 0.4);
        assert_eq!(z.total, 3);
        assert!(z.zones.len() >= 2, "the far tile starts its own zone");
        assert_eq!(z.zone_of[0], z.zone_of[near.index()], "neighbours merge");
    }

    #[test]
    fn sea_tiles_never_join_a_zone() {
        let (sphere, mut snapshot) = world(&[(0, Cover::Field)]);
        snapshot.tiles[0].terrain = Terrain::Sea;
        let z = cover_zones(&sphere, &snapshot, Cover::Field, 0.4);
        assert_eq!(z.total, 0);
        assert!(z.zones.is_empty());
    }

    #[test]
    fn both_projections_round_trip_through_the_frame() {
        let (sphere, _) = world(&[]);
        let zone = vec![TileId(0), sphere.tile(TileId(0)).neighbors[0]];
        let f = zone_frame(&sphere, &zone, 17);
        assert!((len(f.e1) - 1.0).abs() < 1e-12);
        assert!(dot(f.e1, f.c).abs() < 1e-12, "the frame is tangent");
        assert!(dot(f.e1, f.e2).abs() < 1e-12, "and orthogonal");

        for &id in &zone {
            let p = sphere.tile(id).center;
            let back = f.to3(f.to2(p));
            assert!(len(sub(back, p)) < 1e-12, "gnomonic round trip");
            let back_e = f.to3e(f.to2e(p));
            assert!(len(sub(back_e, p)) < 1e-12, "equal-distance round trip");
        }
        // The centre itself is the fixed point of both.
        assert!(len(sub(f.to3e([0.0, 0.0]), f.c)) < 1e-12);
    }

    #[test]
    fn the_equal_distance_projection_keeps_radial_distance() {
        let (sphere, _) = world(&[]);
        let zone = vec![TileId(0)];
        let f = zone_frame(&sphere, &zone, 17);
        let p = sphere.tile(sphere.tile(TileId(0)).neighbors[0]).center;
        let rho = libm::acos(dot(p, f.c).clamp(-1.0, 1.0));
        let q = f.to2e(p);
        assert!(
            (hypot2(q[0], q[1]) - rho).abs() < 1e-12,
            "the plane distance is the angle itself"
        );
        // Gnomonic stretches it, which is the whole reason for the variant.
        let g = f.to2(p);
        assert!(hypot2(g[0], g[1]) > hypot2(q[0], q[1]));
    }

    #[test]
    fn ring_normal_points_out_of_a_counter_clockwise_ring() {
        let r = [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let n = ring_normal(&r);
        assert!((n[2] - 1.0).abs() < 1e-12, "got {n:?}");
    }
}
