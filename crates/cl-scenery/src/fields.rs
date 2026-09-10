//! Farmland, prototype section 4b (`buildFields`).
//!
//! Fields are drawn per *zone*, not per tile. Every connected run of field tiles is projected onto
//! one tangent plane, cut into parcels by a randomised binary split, each parcel shaved back to
//! leave a strip of grass around it, and only *then* clipped against the individual hexes.
//!
//! Parcelling before the hex clip is the whole trick: a field that straddles a tile border is cut
//! from a single polygon, so both halves inherit the same crop, the same furrow direction and the
//! same texture phase, and the border between the two tiles is invisible.

use cl_hexsphere::{Frames, HexSphere, facet_plane};
use cl_model::{Cover, MeshData, TileId, WorldSnapshot, hex_rgb};
use cl_noise::Mulberry32;
use cl_noise::hash3i;
use cl_noise::js::{hypot2, round, to_int32};
use cl_noise::vec::{V3, add, cross, dot, mul, norm, sub};
use cl_pixelart::palette::FIELD_CROPS;
use cl_pixelart::{FURROW_PX, field_row_v};

use crate::poly::{
    Poly, clip_to_hull, dedupe, hull_at, mitre_offset, parcel_split, poly_area, poly_thickness,
    trim_convex,
};
use crate::zones::{ZoneFrame, cover_zones, ring_normal, zone_frame};

/// The top face is the parcel minus this margin in world pixels, and the sloped side then grows back
/// *outward* into it, so the grass left showing between two fields is `2 × FIELD_TRIM` minus their
/// two slopes — roughly 2 to 3 world pixels.
pub const FIELD_TRIM: f64 = 2.5;
/// Keep splitting a parcel while it is wider than this, in world pixels. Sized off the reference
/// art, where a field runs roughly 27 × 15; going smaller is a trap, because the sloped sides are a
/// fixed 1 to 1.5 px and a 12 px parcel ends up more edge than field.
pub const PARCEL_MAX: f64 = 34.0;
/// ...but never leave a strip thinner than this.
pub const PARCEL_MIN: f64 = 18.0;
/// Drop a piece smaller than this, in square world pixels.
pub const SLIVER_PX2: f64 = 2.0;
/// ...or thinner than this, which is a sub-pixel scratch.
pub const THIN_PX: f64 = 0.35;
/// And drop a whole field smaller than about 5 × 5 world pixels.
pub const FIELD_MIN_PX2: f64 = 26.0;
/// Radians: one zone never spans more sky than this.
pub const FARM_SPAN: f64 = 0.40;
/// Share of fields that get a fence.
pub const POST_CHANCE: f64 = 0.15;
/// World pixels between posts, measured along the border.
pub const POST_STEP: f64 = 6.0;
/// Width of a post in world pixels.
pub const POST_W: f64 = 1.0;
/// Height of a post in world pixels.
pub const POST_TALL: f64 = 3.0;
/// Lit top of a post.
pub const POST_TOP: &str = "#b0803f";
/// Its shaded sides.
pub const POST_SIDE: &str = "#8a5c2a";
/// Salt of the farmland zone frames.
pub const FIELD_SALT: i32 = 17;

/// The farmland of one world.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fields {
    /// Tops and sloped sides, sampling the field strip.
    pub surface: MeshData,
    /// Fence posts, untextured; empty when no field drew a fence.
    pub posts: MeshData,
    /// How many zones the field tiles fell into.
    pub zones: usize,
    /// How many parcels were actually drawn.
    pub parcels: usize,
    /// How many of those got a fence.
    pub fences: usize,
    /// How many tiles carry fields altogether.
    pub tiles: usize,
}

/// A colour as a 0–1 triple, the way a vertex colour wants it.
fn tone(hex: &str) -> [f32; 3] {
    let c = hex_rgb(hex);
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
    ]
}

/// One piece of a parcel: the polygon left inside one hex, with what is needed to lift it.
struct Bit {
    piece: Poly,
    tags: Vec<bool>,
    /// Facet plane offset and normal of the tile.
    plane_d: f64,
    plane_n: V3,
    radius: f64,
    area: f64,
}

/// The prototype's `buildFields`. `None` when no tile carries a field.
///
/// # Panics
/// If `snapshot`, `frames` and `sphere` disagree on the tile count.
pub fn build_fields(
    sphere: &HexSphere,
    frames: &Frames,
    snapshot: &WorldSnapshot,
    px: f64,
) -> Option<Fields> {
    assert_eq!(
        snapshot.tiles.len(),
        sphere.len(),
        "one tile state per tile"
    );
    assert_eq!(frames.tiles.len(), sphere.len(), "one frame per tile");
    let zoning = cover_zones(sphere, snapshot, Cover::Field, FARM_SPAN);
    if zoning.total == 0 {
        return None;
    }

    let mut surface = MeshData::default();
    let mut posts = MeshData::default();
    let trim = FIELD_TRIM * px;
    let furrow_len = f64::from(FURROW_PX) * px;
    let hair = 0.06 * px;
    let post_top = tone(POST_TOP);
    let post_side = tone(POST_SIDE);
    let mut parcel_count = 0;
    let mut fence_count = 0;

    for zone in &zoning.zones {
        let frame = zone_frame(sphere, zone, FIELD_SALT);

        // Each tile's hex in the plane, with an edge tag per side: 0 where the field carries on
        // into the next hex, so no side is built there.
        let mut hulls: Vec<Poly> = Vec::with_capacity(zone.len());
        let mut hull_tags: Vec<Vec<bool>> = Vec::with_capacity(zone.len());
        for &id in zone {
            let t = sphere.tile(id);
            let mut h: Poly = t.corners.iter().map(|&c| frame.to2(c)).collect();
            let mut g: Vec<bool> = (0..t.sides())
                .map(|k| {
                    let j = t.edge_neighbors[k];
                    zoning.zone_of[j.index()] != zoning.zone_of[id.index()]
                })
                .collect();
            if poly_area(&h) < 0.0 {
                // Paranoia: keep the clipper's counter-clockwise winding.
                let n = h.len();
                let g2: Vec<bool> = (0..n).map(|k| g[(n - 2 - k + n) % n]).collect();
                h.reverse();
                g = g2;
            }
            hulls.push(h);
            hull_tags.push(g);
        }

        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for h in &hulls {
            for q in h {
                if q[0] < x0 {
                    x0 = q[0];
                }
                if q[0] > x1 {
                    x1 = q[0];
                }
                if q[1] < y0 {
                    y0 = q[1];
                }
                if q[1] > y1 {
                    y1 = q[1];
                }
            }
        }
        // Overshoot the zone, so the cut lines are not pinned to its own outline and parcels run
        // off the edge the way real farmland does.
        let pad = PARCEL_MAX * px * 0.6;
        let boxed: Poly = vec![
            [x0 - pad, y0 - pad],
            [x1 + pad, y0 - pad],
            [x1 + pad, y1 + pad],
            [x0 - pad, y1 + pad],
        ];
        let mut rnd = Mulberry32::from_i32(
            to_int32(f64::from(zone[0].0) * 2_654_435_761.0)
                ^ to_int32(zone.len() as f64 * 40503.0),
        );
        let mut parcels = Vec::new();
        parcel_split(
            &boxed,
            &mut rnd,
            PARCEL_MAX * px,
            PARCEL_MIN * px,
            &mut parcels,
            0,
        );

        for parcel in &parcels {
            let foot = trim_convex(parcel, trim);
            if foot.len() < 3 {
                continue;
            }

            // Crop, tone, furrow direction and fence belong to the PARCEL, so every hex it touches
            // renders the same field rather than a patchwork per tile. Keyed on the centroid
            // quantised to an eighth of a world pixel, through the integer hash: the sine-based
            // hash2 clusters badly on inputs this correlated and skewed the crop mix by a third.
            let mut cx = 0.0;
            let mut cy = 0.0;
            for q in &foot {
                cx += q[0];
                cy += q[1];
            }
            cx /= foot.len() as f64;
            cy /= foot.len() as f64;
            let qx = round(cx / px * 8.0) as i32;
            let qy = round(cy / px * 8.0) as i32;
            let rh = |s: i32| hash3i(qx, qy, s);

            let mut ci = FIELD_CROPS.len() - 1;
            let mut acc = 0.0;
            let h0 = rh(1);
            for (i, crop) in FIELD_CROPS.iter().enumerate() {
                acc += crop.w;
                if h0 < acc {
                    ci = i;
                    break;
                }
            }
            let crop = FIELD_CROPS[ci];
            // A 45-degree wedge, not a wall.
            let lift = crop.h * px;
            let slope = crop.h * px;
            let shade = (0.94 + rh(2) * 0.12) as f32;
            let top_row = field_row_v(2 * ci as u32);
            let side_row = field_row_v(2 * ci as u32 + 1);

            // Furrows run along the longest side, or across it on a coin flip.
            let mut fd = [1.0, 0.0];
            let mut best_l = -1.0;
            for i in 0..foot.len() {
                let a = foot[i];
                let b = foot[(i + 1) % foot.len()];
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let l = hypot2(dx, dy);
                if l > best_l {
                    best_l = l;
                    fd = [dx / l, dy / l];
                }
            }
            if rh(3) < 0.45 {
                fd = [-fd[1], fd[0]];
            }
            // u must climb ACROSS the lines for them to run along fd.
            let fnormal = norm(add(mul(frame.e1, -fd[1]), mul(frame.e2, fd[0])));
            let phase = rh(4);
            let u_of = |p: V3| dot(p, fnormal) / furrow_len + phase;

            // Pass one: cut the footprint against every hex it touches.
            let mut bits: Vec<Bit> = Vec::new();
            for (zi, &id) in zone.iter().enumerate() {
                let ones = vec![true; foot.len()];
                let (piece, tags) = clip_to_hull(&foot, &ones, &hulls[zi], &hull_tags[zi]);
                let (piece, tags) = dedupe(&piece, &tags, 1e-7 * px + 1e-12);
                if piece.len() < 3 {
                    continue;
                }
                // Only genuine scratches are dropped, and only on the two tests a sub-pixel shard
                // fails. Anything larger is kept whatever its shape, because a piece is a FRAGMENT
                // of a field: dropping one leaves a hole in the middle rather than trimming an edge.
                let area = poly_area(&piece);
                if area < SLIVER_PX2 * px * px || poly_thickness(&piece) < THIN_PX * px {
                    continue;
                }
                let t = sphere.tile(id);
                let f = &frames.tiles[id.index()];
                let plane = facet_plane(t, f);
                bits.push(Bit {
                    piece,
                    tags,
                    plane_d: plane.d,
                    plane_n: plane.n,
                    radius: f.radius,
                    area,
                });
            }
            if bits.is_empty() {
                continue;
            }
            // A parcel that only grazes the zone survives its per-piece tests but renders as a
            // pointy shard. Judge it once, on the whole parcel, so a field is either drawn or not —
            // never holed in the middle.
            let kept: f64 = bits.iter().map(|b| b.area).sum();
            if kept < FIELD_MIN_PX2 * px * px {
                continue;
            }
            parcel_count += 1;

            // ONE normal for the whole parcel, area-weighted across the hexes it covers. The hexes
            // are separate flat facets with normals a few degrees apart, so shading each fragment by
            // its own facet would draw a crease along the tile seam — exactly the join the merge is
            // supposed to hide. The sloped sides keep their own normals; they really are slanted.
            let mut pn = [0.0, 0.0, 0.0];
            for b in &bits {
                pn = add(pn, mul(b.plane_n, b.area));
            }
            let pn = norm(pn);

            // Pass two: geometry.
            for b in &bits {
                // Find the tile's facet plane along this ray, then raise the vertex along the RAY,
                // not along the facet normal. Two neighbouring facets both contain their shared
                // edge, so on that edge they agree on the distance — and because the lift direction
                // is the ray, a property of the point rather than of the tile asking, both tiles
                // land on the identical vertex. Lifting along each tile's own normal left a step of
                // about 0.22 px at every seam, the normals being some 5 degrees apart.
                let at = |q: [f64; 2], hh: f64| -> V3 {
                    let dir = frame.to3(q);
                    mul(dir, b.plane_d * b.radius / dot(dir, b.plane_n) + hh)
                };
                // The top IS the piece. Two neighbouring pieces were cut from the same footprint by
                // the same hex-edge line, so their tops meet along it exactly and the field reads as
                // one surface.
                let t3: Vec<V3> = b.piece.iter().map(|&q| at(q, lift)).collect();
                let g3: Vec<V3> = mitre_offset(&b.piece, &b.tags, slope, slope * 2.2)
                    .iter()
                    .map(|&q| at(q, hair))
                    .collect();

                let mut mid = [0.0, 0.0, 0.0];
                for &p in &t3 {
                    mid = add(mid, p);
                }
                let mid = mul(mid, 1.0 / t3.len() as f64);
                let u_mid = [u_of(mid), top_row];
                for k in 0..t3.len() {
                    let a = t3[k];
                    let bb = t3[(k + 1) % t3.len()];
                    push(&mut surface, mid, pn, u_mid, shade);
                    push(&mut surface, a, pn, [u_of(a), top_row], shade);
                    push(&mut surface, bb, pn, [u_of(bb), top_row], shade);
                }

                let mut ctr = [0.0, 0.0, 0.0];
                for &p in &g3 {
                    ctr = add(ctr, p);
                }
                let ctr = mul(ctr, 1.0 / g3.len() as f64);
                for k in 0..b.piece.len() {
                    if !b.tags[k] {
                        continue; // hex seam: stays flush
                    }
                    let kn = (k + 1) % b.piece.len();
                    let mut ring = [g3[k], g3[kn], t3[kn], t3[k]];
                    let mut wn = ring_normal(&ring);
                    if dot(wn, sub(mul(add(g3[k], g3[kn]), 0.5), ctr)) < 0.0 {
                        ring = [g3[k], t3[k], t3[kn], g3[kn]];
                        wn = mul(wn, -1.0);
                    }
                    let uu: Vec<[f64; 2]> = ring.iter().map(|&p| [u_of(p), side_row]).collect();
                    for i in [0, 1, 2, 0, 2, 3] {
                        push(&mut surface, ring[i], wn, uu[i], shade);
                    }
                }
            }

            // Fences: a minority of fields get posts, evenly spaced right around the parcel's own
            // border — the field's outline, not the hex grid's.
            if rh(5) >= POST_CHANCE {
                continue;
            }
            fence_count += 1;
            fence(
                &mut posts, sphere, frames, &frame, zone, &hulls, &foot, px, post_top, post_side,
            );
        }
    }

    if surface.positions.is_empty() {
        return None;
    }
    Some(Fields {
        surface,
        posts,
        zones: zoning.zones.len(),
        parcels: parcel_count,
        fences: fence_count,
        tiles: zoning.total,
    })
}

/// One vertex of the field surface.
fn push(mesh: &mut MeshData, p: V3, n: V3, uv: [f64; 2], shade: f32) {
    mesh.push_vertex(p, n);
    mesh.colors.extend([shade, shade, shade]);
    mesh.uvs.extend([uv[0] as f32, uv[1] as f32]);
}

/// Posts around one parcel's outline.
#[allow(clippy::too_many_arguments)]
fn fence(
    posts: &mut MeshData,
    sphere: &HexSphere,
    frames: &Frames,
    frame: &ZoneFrame,
    zone: &[TileId],
    hulls: &[Poly],
    foot: &[[f64; 2]],
    px: f64,
    post_top: [f32; 3],
    post_side: [f32; 3],
) {
    let step = POST_STEP * px;
    let hw = POST_W * px * 0.5;
    let hh = POST_TALL * px;
    let mut carry = step * 0.5;
    for i in 0..foot.len() {
        let a = foot[i];
        let b = foot[(i + 1) % foot.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let l = hypot2(dx, dy);
        if l < 1e-12 {
            continue;
        }
        let ux = dx / l;
        let uy = dy / l;
        let mut s = carry;
        while s < l {
            let q = [a[0] + ux * s, a[1] + uy * s];
            if let Some(zi) = hull_at(hulls, q) {
                let id = zone[zi];
                let t = sphere.tile(id);
                let f = &frames.tiles[id.index()];
                let plane = facet_plane(t, f);
                let dir = frame.to3(q);
                let base = mul(dir, plane.d * f.radius / dot(dir, plane.n));
                // The same ray-based up as the fields.
                let up = dir;
                let al = add(mul(frame.e1, ux), mul(frame.e2, uy));
                let al = norm(sub(al, mul(up, dot(al, up))));
                let cr = cross(up, al);
                let corner = |sx: f64, sy: f64, sz: f64| -> V3 {
                    add(
                        base,
                        add(add(mul(al, hw * sx), mul(cr, hw * sy)), mul(up, sz * hh)),
                    )
                };
                let bot = [
                    corner(-1.0, -1.0, 0.0),
                    corner(1.0, -1.0, 0.0),
                    corner(1.0, 1.0, 0.0),
                    corner(-1.0, 1.0, 0.0),
                ];
                let top = [
                    corner(-1.0, -1.0, 1.0),
                    corner(1.0, -1.0, 1.0),
                    corner(1.0, 1.0, 1.0),
                    corner(-1.0, 1.0, 1.0),
                ];
                quad(posts, &top, up, post_top);
                quad(
                    posts,
                    &[bot[0], bot[1], top[1], top[0]],
                    mul(cr, -1.0),
                    post_side,
                );
                quad(posts, &[bot[1], bot[2], top[2], top[1]], al, post_side);
                quad(posts, &[bot[2], bot[3], top[3], top[2]], cr, post_side);
                quad(
                    posts,
                    &[bot[3], bot[0], top[0], top[3]],
                    mul(al, -1.0),
                    post_side,
                );
            }
            s += step;
        }
        carry = s - l;
    }
}

/// Two triangles of a post's face.
fn quad(mesh: &mut MeshData, r: &[V3; 4], n: V3, tone: [f32; 3]) {
    for k in [0, 1, 2, 0, 2, 3] {
        mesh.push_vertex(r[k], n);
        mesh.colors.extend(tone);
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_hexsphere::compute_tile_frames;
    use cl_model::{Terrain, TileState};
    use cl_noise::vec::len;

    use super::*;

    #[test]
    fn a_world_with_no_farmland_builds_nothing() {
        let sphere = HexSphere::build(3);
        let snapshot = WorldSnapshot {
            frequency: 3,
            seed: 1,
            tiles: vec![
                TileState {
                    terrain: Terrain::Land,
                    ..TileState::default()
                };
                sphere.len()
            ],
        };
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        assert!(build_fields(&sphere, &frames, &snapshot, frames.px).is_none());
    }

    #[test]
    fn farmland_is_a_valid_mesh_whose_counts_add_up() {
        let sphere = HexSphere::build(3);
        let mut snapshot = WorldSnapshot {
            frequency: 3,
            seed: 1,
            tiles: vec![
                TileState {
                    terrain: Terrain::Land,
                    ..TileState::default()
                };
                sphere.len()
            ],
        };
        // One tile and its whole ring: a zone big enough to parcel.
        snapshot.tiles[0].cover = Cover::Field;
        for &j in &sphere.tile(TileId(0)).neighbors {
            snapshot.tiles[j.index()].cover = Cover::Field;
        }
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let f = build_fields(&sphere, &frames, &snapshot, frames.px).expect("a zone");
        assert_eq!(f.tiles, 1 + sphere.tile(TileId(0)).neighbors.len());
        assert_eq!(f.zones, 1, "the ring is connected, so it is one zone");
        assert!(f.parcels > 0);
        assert!(f.surface.validate().is_ok(), "{:?}", f.surface.validate());
        assert!(f.posts.validate().is_ok());
        assert_eq!(f.surface.uvs.len(), f.surface.vertex_count() * 2);
        assert_eq!(f.surface.colors.len(), f.surface.positions.len());
        assert!(f.posts.uvs.is_empty(), "posts carry no texture");
        // Fences are a minority, and each post is five quads.
        assert_eq!(f.posts.vertex_count() % 30, 0);
        if f.fences == 0 {
            assert!(f.posts.is_empty());
        }
    }

    #[test]
    fn every_field_vertex_sits_above_the_land_shell() {
        let sphere = HexSphere::build(3);
        let mut snapshot = WorldSnapshot {
            frequency: 3,
            seed: 1,
            tiles: vec![
                TileState {
                    terrain: Terrain::Land,
                    ..TileState::default()
                };
                sphere.len()
            ],
        };
        snapshot.tiles[0].cover = Cover::Field;
        for &j in &sphere.tile(TileId(0)).neighbors {
            snapshot.tiles[j.index()].cover = Cover::Field;
        }
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let f = build_fields(&sphere, &frames, &snapshot, frames.px).expect("a zone");
        for v in f.surface.positions.chunks(3) {
            let r = len([f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]);
            assert!(
                r > 0.9 && r < 1.1,
                "a field vertex at radius {r} is off the shell"
            );
        }
    }
}
