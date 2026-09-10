//! Woods, prototype section 4c (`buildForest`): a low-poly mesh of individual trees, not a stamped
//! slab.
//!
//! Every tree is a small faceted crown — three tapering rings plus an apex, five or six sides, each
//! vertex nudged off the ideal cone so no two trees match. The pixel-art look comes from the
//! shading, not a texture: each band of the crown takes one flat colour from a four-step ramp and a
//! per-facet brightness jitter stands in for dithering.
//!
//! Trees are planted on a jittered grid whose pitch is smaller than a crown, so neighbouring crowns
//! interpenetrate and the wood reads as one bumpy mass. A tree may only root on a forest tile, but
//! its crown is never clipped, so the treeline spills past the hex border and the silhouette stays
//! organic. A few stunted bushes just outside the wood break the outline up further.

use cl_hexsphere::{Frames, HexSphere, facet_plane};
use cl_model::{Cover, MeshData, TileId, WorldSnapshot, hex_rgb};
use cl_noise::vec::{V3, add, cross, dot, len, mul, norm, sub};
use cl_noise::{fbm3, hash3i, js};

use crate::zones::{ZoneFrame, Zoning, cover_zones, in_hull, poly_area};

/// The commonest crown green, about half the wood.
pub const CANOPY_BODY: &str = "#3a7a26";
/// Hue patches laid over [`CANOPY_BODY`] in broad zones. The prototype's `CANOPY.rim` and
/// `CANOPY.side` are reference tones only: the dark skirt and flank are scaled from the base here.
pub const CANOPY_ZONES: [&str; 2] = ["#6a8d25", "#78a226"];
/// Width of one tree crown, in world pixels.
pub const CROWN_PX: f64 = 10.0;
/// Planting pitch, in world pixels: below [`CROWN_PX`], so crowns overlap.
pub const CROWN_STEP: f64 = 5.0;
/// How far a tree wanders off its slot, in world pixels.
pub const CROWN_JIT: f64 = 2.0;
/// Typical crown height, in world pixels.
pub const TREE_H_PX: f64 = 7.0;
/// How deep a crown is buried, in world pixels, so none ever hovers.
pub const TREE_SINK_PX: f64 = 1.2;
/// How far outside the wood a stray may root, in world pixels.
pub const BUSH_MARGIN_PX: f64 = 6.0;
/// Chance of a stray per slot in that margin, so bushes stay rare.
pub const BUSH_CHANCE: f64 = 0.14;
/// Frequency of the hue patches: about two tiles across.
pub const CANOPY_ZONE_F: f64 = 7.0;
/// Smallest crown scale. The spread runs wide: a wood of near-identical blobs reads as texture.
pub const TREE_MIN: f64 = 0.55;
/// Largest crown scale.
pub const TREE_MAX: f64 = 1.60;
/// Frequency of the vigour field. Half the size spread is per-tree and half comes from this slow
/// field, so big and small trees fall into stands rather than salt-and-pepper.
pub const VIGOUR_F: f64 = 11.0;
/// Smallest understory disc, in world pixels; below this the merged blob breaks up.
pub const FLOOR_R_MIN: f64 = 4.9;
/// How much wider than its crown a disc always is.
pub const FLOOR_GROW: f64 = 1.15;
/// How far a disc floats above the facet, in world pixels.
pub const FLOOR_LIFT_PX: f64 = 0.5;
/// How much darker than the crown a disc is.
pub const FLOOR_SHADE: f64 = 0.40;
/// Half-angle cap of a forest zone, in radians. Wider than the farmland cap: farmland needs the
/// gnomonic projection for exact polygon clipping and that distorts fast, while the planting grid
/// uses the equal-distance mapping, which only loses `sin(p)/p` across — 6 % out here.
pub const FOREST_SPAN: f64 = 0.60;

/// Crown profile as height and radius fractions: the bottom ring is tucked in so the skirt reads as
/// the dark outline, widest at two thirds, doming to the apex.
const RINGS: [Ring; 4] = [
    Ring { y: -0.10, r: 0.55 },
    Ring { y: 0.30, r: 0.92 },
    Ring { y: 0.60, r: 1.00 },
    Ring { y: 0.85, r: 0.60 },
];
/// Flat shade of each band of faces, skirt to tip.
const BAND_SH: [f64; 4] = [0.38, 0.62, 0.88, 1.18];
/// Sides of an understory disc. Seven, not six: an odd count keeps the disc from lining up with the
/// hex grid underneath it.
const FLOOR_SIDES: usize = 7;
/// Hash salt of the zone frames, shared with the other cover renderers.
const ZONE_SALT: i32 = 29;

/// One ring of a crown: height and radius as fractions of the tree's height and radius.
struct Ring {
    y: f64,
    r: f64,
}

/// The wood of one world: every crown and understory disc in a single mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct Forest {
    /// Crowns and floor discs, flat-shaded with vertex colours and no texture.
    pub surface: MeshData,
    /// Connected zones the trees were planted in.
    pub zones: usize,
    /// Tiles carrying forest cover.
    pub tiles: usize,
    /// Trees planted, strays included.
    pub crowns: usize,
}

/// The prototype's `buildForest`. `None` where the world carries no forest at all.
///
/// `px` is one world pixel, `frames.px` in practice; `seed` is the world seed.
///
/// # Panics
/// If `snapshot` and `sphere` disagree on the tile count.
pub fn build_forest(
    sphere: &HexSphere,
    frames: &Frames,
    snapshot: &WorldSnapshot,
    px: f64,
    seed: f64,
) -> Option<Forest> {
    assert_eq!(
        snapshot.tiles.len(),
        sphere.len(),
        "one tile state per tile"
    );
    assert_eq!(frames.tiles.len(), sphere.len(), "one frame per tile");
    let zoning = cover_zones(sphere, snapshot, Cover::Forest, FOREST_SPAN);
    if zoning.total == 0 {
        return None;
    }
    let [t1, t2] = canopy_bands(seed);
    let base_cols: [[f64; 3]; 3] = [CANOPY_BODY, CANOPY_ZONES[0], CANOPY_ZONES[1]].map(|h| {
        let c = hex_rgb(h);
        [
            f64::from(c[0]) / 255.0,
            f64::from(c[1]) / 255.0,
            f64::from(c[2]) / 255.0,
        ]
    });
    let seed_i = js::to_int32(seed);

    let mut mesh = MeshData::default();
    let mut crowns = 0;

    for (zi, zone) in zoning.zones.iter().enumerate() {
        let frame = ZoneFrame::new(sphere, zone, ZONE_SALT);
        let hulls: Vec<Vec<[f64; 2]>> = zone
            .iter()
            .map(|&id| hull_of(sphere, &frame, id).0)
            .collect();
        let bushes = bush_ring(sphere, snapshot, &zoning, &frame, zone, zi as i32);

        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for h in hulls.iter().chain(bushes.iter().map(|b| &b.hull)) {
            for q in h {
                x0 = x0.min(q[0]);
                x1 = x1.max(q[0]);
                y0 = y0.min(q[1]);
                y1 = y1.max(q[1]);
            }
        }
        let step = CROWN_STEP * px;
        let margin = BUSH_MARGIN_PX * px;
        let gi0 = (x0 / step).floor() as i32 - 1;
        let gi1 = (x1 / step).ceil() as i32 + 1;
        let gj0 = (y0 / step).floor() as i32 - 1;
        let gj1 = (y1 / step).ceil() as i32 + 1;

        for sj in gj0..=gj1 {
            for si in gi0..=gi1 {
                let q = [
                    (f64::from(si) + 0.5 + (hash3i(si, sj, seed_i) - 0.5) * CROWN_JIT / CROWN_STEP)
                        * step,
                    (f64::from(sj)
                        + 0.5
                        + (hash3i(si, sj, seed_i.wrapping_add(977)) - 0.5) * CROWN_JIT
                            / CROWN_STEP)
                        * step,
                ];
                let mut bush = false;
                let mut root = hulls.iter().position(|h| in_hull(h, q)).map(|i| zone[i]);
                if root.is_none() {
                    // Outside the wood: the ring of land around it, where strays root.
                    root = bushes
                        .iter()
                        .find(|b| roots_in_margin(b, q, margin))
                        .map(|b| b.tile);
                    if root.is_none() {
                        continue;
                    }
                    bush = true;
                    if hash3i(si, sj, seed_i.wrapping_add(2311)) > BUSH_CHANCE {
                        continue;
                    }
                }
                let tile = root.expect("slots with nothing to root on are skipped");

                let kz = seed_i.wrapping_add((zi as i32).wrapping_mul(733));
                let t = &sphere.tiles[tile.index()];
                let tf = &frames.tiles[tile.index()];
                let plane = facet_plane(t, tf);
                let dir = frame.to3e(q);
                // Root on the tile's facet plane along this ray — the same rule the fields use, so
                // a tree near a hex seam agrees with its neighbour.
                let mut g = mul(dir, plane.d * tf.radius / dot(dir, plane.n));
                let up = norm(g);
                let mut a1 = sub(frame.e1, mul(up, dot(frame.e1, up)));
                if len(a1) < 1e-6 {
                    a1 = sub(frame.e2, mul(up, dot(frame.e2, up)));
                }
                let a1 = norm(a1);
                let rot = hash3i(si, sj, kz.wrapping_add(31)) * std::f64::consts::PI * 2.0;
                let a1r = add(mul(a1, js::cos(rot)), mul(cross(up, a1), js::sin(rot)));
                let a2r = cross(up, a1r); // (a1r, a2r, up) right-handed

                let h4 = hash3i(si, sj, kz.wrapping_add(57));
                let h5 = hash3i(si, sj, kz.wrapping_add(91));
                let h6 = hash3i(si, sj, kz.wrapping_add(137));

                // Half the size spread is this tree's own roll, half is a slow field, so big trees
                // gather into stands instead of peppering the wood evenly.
                let vig = fbm3(dir, f64::from(seed_i ^ 0x5bf), 2, VIGOUR_F);
                let clump = 1.0f64.min(0.0f64.max((vig - 0.34) / 0.32));
                let sz = TREE_MIN + (TREE_MAX - TREE_MIN) * (0.45 * clump + 0.55 * h4);

                let mut r = CROWN_PX * 0.5 * px * sz;
                let mut h = TREE_H_PX * px * sz * (0.90 + h5 * 0.20);
                let mut bright = 0.96 + hash3i(si, sj, kz.wrapping_add(211)) * 0.08;
                if bush {
                    // Strays: saplings to small trees.
                    r *= 0.45 + h6 * 0.45;
                    h *= 0.50 + h6 * 0.45;
                } else if h6 < 0.07 {
                    // One emergent per stand.
                    h *= 1.35;
                    r *= 0.88;
                    bright *= 0.92;
                }
                g = sub(g, mul(up, TREE_SINK_PX * px * if bush { 0.6 } else { 1.0 }));

                // Hue patches from the global field, so colour never steps at a zone join.
                let v = fbm3(dir, seed, 2, CANOPY_ZONE_F);
                let base = base_cols[if v < t1 {
                    0
                } else if v < t2 {
                    1
                } else {
                    2
                }];
                let sides = 5 + (hash3i(si, sj, kz.wrapping_add(257)) * 1.999) as usize;

                // Inside the wood the disc has a floor size, which is what welds the canopy
                // together. A stray keeps its own small disc so it reads as one tree standing
                // alone, not as an outlier of the mass.
                let fr = if bush {
                    r * FLOOR_GROW
                } else {
                    r.max(FLOOR_R_MIN * px) * FLOOR_GROW
                };
                let slot = Slot {
                    g,
                    a1: a1r,
                    a2: a2r,
                    up,
                    kx: si,
                    ky: sj,
                    kz,
                };
                add_floor(&mut mesh, &slot, fr, base, px);
                add_tree(&mut mesh, &slot, r, h, sides, base, bright);
                crowns += 1;
            }
        }
    }

    if mesh.is_empty() {
        return None;
    }
    Some(Forest {
        surface: mesh,
        zones: zoning.zones.len(),
        tiles: zoning.total,
        crowns,
    })
}

/// Quantiles of the zoning noise taken over the whole sphere, not per zone, so the colour field is
/// one continuous thing that happens to be sampled by several zones. Directions come off a
/// deterministic spiral, which covers the sphere evenly for any count.
fn canopy_bands(seed: f64) -> [f64; 2] {
    const N: usize = 512;
    // A `Float32Array` in the prototype, and the rounding shows: a threshold is compared against
    // full `f64` samples later, so the band edges must be the rounded values.
    let mut v = [0.0f32; N];
    for (i, slot) in v.iter_mut().enumerate() {
        let z = 1.0 - 2.0 * (i as f64 + 0.5) / N as f64;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let a = i as f64 * 2.39996323;
        *slot = fbm3([r * js::cos(a), r * js::sin(a), z], seed, 2, CANOPY_ZONE_F) as f32;
    }
    v.sort_by(|a, b| a.partial_cmp(b).expect("noise is finite"));
    [
        f64::from(v[(N as f64 * 0.50) as usize]),
        f64::from(v[(N as f64 * 0.78) as usize]),
    ]
}

/// A tile just outside a zone that a stray may root on, with the edges it shares with the zone.
struct Bush {
    tile: TileId,
    hull: Vec<[f64; 2]>,
    /// For each edge of the hull, whether the tile across it belongs to the zone.
    zone_edge: Vec<bool>,
}

/// The tile's corners in the zone's plane, counter-clockwise, and whether the winding was flipped
/// to get there.
fn hull_of(sphere: &HexSphere, frame: &ZoneFrame, id: TileId) -> (Vec<[f64; 2]>, bool) {
    let mut h: Vec<[f64; 2]> = sphere.tiles[id.index()]
        .corners
        .iter()
        .map(|&p| frame.to2e(p))
        .collect();
    let flipped = poly_area(&h) < 0.0;
    if flipped {
        h.reverse();
    }
    (h, flipped)
}

/// One ring of land around the wood, for the stray bushes. Each bush tile remembers which of its
/// edges touch the zone; a bush may only root within [`BUSH_MARGIN_PX`] of one of those edges.
fn bush_ring(
    sphere: &HexSphere,
    snapshot: &WorldSnapshot,
    zoning: &Zoning,
    frame: &ZoneFrame,
    zone: &[TileId],
    zi: i32,
) -> Vec<Bush> {
    let mut seen = vec![false; sphere.len()];
    for &id in zone {
        seen[id.index()] = true;
    }
    let mut bushes = Vec::new();
    for &id in zone {
        for &j in &sphere.tiles[id.index()].neighbors {
            if seen[j.index()] {
                continue;
            }
            seen[j.index()] = true;
            if snapshot.tiles[j.index()].terrain.level() < 0 {
                continue; // nothing roots at sea
            }
            let n = &sphere.tiles[j.index()];
            let sides = n.sides();
            let mut zone_edge: Vec<bool> = (0..sides)
                .map(|k| zoning.zone_of[n.edge_neighbors[k].index()] == zi)
                .collect();
            if !zone_edge.iter().any(|&x| x) {
                continue;
            }
            let (hull, flipped) = hull_of(sphere, frame, j);
            if flipped {
                // Reversing the corners renumbers the edges: edge k of the flipped ring runs from
                // corner k to k+1 of the reversed list, which was edge N-2-k of the original.
                zone_edge = (0..sides)
                    .map(|k| zone_edge[(sides * 2 - 2 - k) % sides])
                    .collect();
            }
            bushes.push(Bush {
                tile: j,
                hull,
                zone_edge,
            });
        }
    }
    bushes
}

/// Is the slot inside this bush tile and within `margin` of one of its edges onto the zone?
fn roots_in_margin(b: &Bush, q: [f64; 2], margin: f64) -> bool {
    let mut near = false;
    for k in 0..b.hull.len() {
        let a = b.hull[k];
        let c = b.hull[(k + 1) % b.hull.len()];
        let dx = c[0] - a[0];
        let dy = c[1] - a[1];
        let l = js::hypot2(dx, dy);
        let l = if l > 0.0 { l } else { 1.0 };
        let ins = (-(q[0] - a[0]) * dy + (q[1] - a[1]) * dx) / l; // inset from edge k
        if ins < -1e-9 {
            return false;
        }
        if b.zone_edge[k] && ins <= margin {
            near = true;
        }
    }
    near
}

/// One planting slot: where a tree stands, its tangent frame and its hash key.
struct Slot {
    g: V3,
    a1: V3,
    a2: V3,
    up: V3,
    kx: i32,
    ky: i32,
    kz: i32,
}

/// Appends one flat-shaded triangle. Rings are built counter-clockwise seen from above with
/// `(a1, a2, up)` right-handed, which makes this winding outward-facing for every band, the fan and
/// the cap.
fn tri(mesh: &mut MeshData, a: V3, b: V3, c: V3, col: [f64; 3]) {
    let n = norm(cross(sub(b, a), sub(c, a)));
    for p in [a, b, c] {
        mesh.push_vertex(p, n);
        mesh.colors
            .extend([col[0] as f32, col[1] as f32, col[2] as f32]);
    }
}

/// A dark disc laid on the ground under one crown, a little wider than the crown itself.
/// Overlapping discs merge into one blob with a scalloped edge, so gaps between crowns show
/// shadowed floor instead of bright grass and the wood gets a single outline instead of one rim per
/// tree. Radius wobbles per disc so the merged outline is lumpy rather than a chain of circles.
fn add_floor(mesh: &mut MeshData, s: &Slot, r: f64, base: [f64; 3], px: f64) {
    let c = [
        base[0] * FLOOR_SHADE,
        base[1] * FLOOR_SHADE,
        base[2] * FLOOR_SHADE,
    ];
    let mid = add(s.g, mul(s.up, FLOOR_LIFT_PX * px));
    let spin = hash3i(s.kx, s.ky, s.kz.wrapping_add(61)) * std::f64::consts::PI * 2.0;
    let mut pts = Vec::with_capacity(FLOOR_SIDES);
    for i in 0..FLOOR_SIDES {
        let a = i as f64 / FLOOR_SIDES as f64 * std::f64::consts::PI * 2.0 + spin;
        let rr = r
            * (1.0
                + (hash3i(
                    s.kx.wrapping_mul(7).wrapping_add(i as i32),
                    s.ky.wrapping_mul(11),
                    s.kz.wrapping_add(83),
                ) - 0.5)
                    * 0.22);
        pts.push(add(
            mid,
            add(mul(s.a1, js::cos(a) * rr), mul(s.a2, js::sin(a) * rr)),
        ));
    }
    for i in 0..FLOOR_SIDES {
        tri(mesh, mid, pts[i], pts[(i + 1) % FLOOR_SIDES], c);
    }
}

/// One crown: [`RINGS`] tapering rings plus an apex, flat-shaded band by band with a per-facet
/// brightness jitter standing in for dithering.
fn add_tree(
    mesh: &mut MeshData,
    s: &Slot,
    r: f64,
    h: f64,
    sides: usize,
    base: [f64; 3],
    bright: f64,
) {
    let mut rings: Vec<Vec<V3>> = Vec::with_capacity(RINGS.len());
    for (j, ring) in RINGS.iter().enumerate() {
        let mut pts = Vec::with_capacity(sides);
        // Slight twist per ring.
        let tw = j as f64 * 0.45 + hash3i(s.kx.wrapping_add(j as i32), s.ky, s.kz) * 0.6;
        for i in 0..sides {
            let a = i as f64 / sides as f64 * std::f64::consts::PI * 2.0 + tw;
            // Off the perfect cone.
            let jr = 1.0
                + (hash3i(
                    s.kx.wrapping_mul(3).wrapping_add(i as i32),
                    s.ky.wrapping_mul(5).wrapping_add(j as i32),
                    s.kz,
                ) - 0.5)
                    * 0.18;
            let rr = r * ring.r * jr;
            pts.push(add(
                s.g,
                add(
                    mul(s.up, h * ring.y),
                    add(mul(s.a1, js::cos(a) * rr), mul(s.a2, js::sin(a) * rr)),
                ),
            ));
        }
        rings.push(pts);
    }
    // A little lean.
    let apex = add(
        s.g,
        add(
            mul(s.up, h),
            add(
                mul(
                    s.a1,
                    (hash3i(s.kx, s.ky, s.kz.wrapping_add(401)) - 0.5) * r * 0.6,
                ),
                mul(
                    s.a2,
                    (hash3i(s.kx, s.ky, s.kz.wrapping_add(407)) - 0.5) * r * 0.6,
                ),
            ),
        ),
    );
    let shade = |sh: f64, warm: f64, f: i32| -> [f64; 3] {
        // A facet is a dither cell.
        let j = bright
            * (0.975
                + hash3i(
                    s.kx.wrapping_add(f),
                    s.ky.wrapping_sub(f),
                    s.kz.wrapping_add(501),
                ) * 0.05);
        [
            1.0f64.min(base[0] * sh * j + warm * 0.09),
            1.0f64.min(base[1] * sh * j + warm * 0.11),
            1.0f64.min(base[2] * sh * j),
        ]
    };

    let mut f = 0;
    for j in 0..RINGS.len() - 1 {
        let warm = if j == RINGS.len() - 2 { 0.4 } else { 0.0 };
        for i in 0..sides {
            let i2 = (i + 1) % sides;
            let a = rings[j][i];
            let b = rings[j][i2];
            let c = rings[j + 1][i2];
            let d = rings[j + 1][i];
            tri(mesh, a, b, c, shade(BAND_SH[j], warm, f));
            f += 1;
            tri(mesh, a, c, d, shade(BAND_SH[j], warm, f));
            f += 1;
        }
    }
    let top = RINGS.len() - 1;
    for i in 0..sides {
        tri(
            mesh,
            rings[top][i],
            rings[top][(i + 1) % sides],
            apex,
            shade(BAND_SH[top], 1.0, f),
        );
        f += 1;
    }
    // Underside cap, so a crown caught at the horizon is never hollow. One shade for the whole cap,
    // so the facet counter stops here.
    let c0 = add(s.g, mul(s.up, h * RINGS[0].y));
    let cc = shade(BAND_SH[0], 0.0, f);
    for i in 0..sides {
        tri(mesh, rings[0][(i + 1) % sides], rings[0][i], c0, cc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cl_hexsphere::compute_tile_frames;
    use cl_model::{Terrain, TileState};

    fn world(frequency: u8, forest: &[usize]) -> (HexSphere, WorldSnapshot) {
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
        (
            HexSphere::build(frequency),
            WorldSnapshot {
                frequency,
                seed: 1234,
                tiles,
            },
        )
    }

    #[test]
    fn a_world_without_forest_builds_nothing() {
        let (sphere, snapshot) = world(4, &[]);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        assert!(build_forest(&sphere, &frames, &snapshot, frames.px, 1234.0).is_none());
    }

    #[test]
    fn one_forest_tile_grows_a_consistent_mesh_of_whole_trees() {
        let (sphere, snapshot) = world(4, &[0]);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let forest = build_forest(&sphere, &frames, &snapshot, frames.px, 1234.0).expect("a wood");
        assert_eq!(forest.zones, 1);
        assert_eq!(forest.tiles, 1);
        assert!(forest.crowns > 0, "a tile the size of a hex holds trees");
        assert!(forest.surface.validate().is_ok());
        // Every tree is one disc (7 triangles) plus a crown of 8 triangles per side, 5 or 6 sides.
        let tris = forest.surface.triangle_count();
        assert!(
            tris >= forest.crowns * (7 + 8 * 5) && tris <= forest.crowns * (7 + 8 * 6),
            "{tris} triangles for {} crowns",
            forest.crowns
        );
        assert_eq!(forest.surface.colors.len(), forest.surface.positions.len());
    }

    #[test]
    fn crowns_stand_above_the_planet_and_lean_no_further_than_a_crown() {
        let (sphere, snapshot) = world(4, &[0]);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let forest = build_forest(&sphere, &frames, &snapshot, frames.px, 1234.0).expect("a wood");
        let radius = frames.tiles[0].radius;
        let top = TREE_H_PX * TREE_MAX * 1.35 * frames.px;
        for v in forest.surface.positions.as_chunks::<3>().0 {
            let d = len([f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]);
            assert!(
                d > radius - TREE_SINK_PX * TREE_MAX * frames.px * 2.0 && d < radius + top,
                "vertex {d} outside the canopy shell"
            );
        }
    }

    #[test]
    fn the_canopy_bands_split_the_field_in_half_and_at_the_top_fifth() {
        let bands = canopy_bands(1234.0);
        assert!(bands[0] < bands[1], "{bands:?} must be ordered");
        assert!(
            bands[0] > 0.0 && bands[1] < 1.0,
            "{bands:?} inside the range"
        );
    }
}
