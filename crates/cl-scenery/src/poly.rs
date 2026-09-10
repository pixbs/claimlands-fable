//! The 2D convex polygon kit the cover renderers cut their layouts with, prototype section 4b.
//!
//! Every edge carries a *tag*: `true` means "this is the outside of a field and needs a sloped
//! side", `false` means "this is only where one hex hands over to the next, keep it flush". Tags are
//! what let one routine handle both the parcel outline and the hex cut without a second pass.

use cl_noise::Mulberry32;
use cl_noise::js::{cos, hypot2, sin};

/// A polygon as a ring of points, counter-clockwise.
pub type Poly = Vec<[f64; 2]>;

/// Twice the signed area, halved: positive for a counter-clockwise ring.
pub fn poly_area(p: &[[f64; 2]]) -> f64 {
    let mut a = 0.0;
    for i in 0..p.len() {
        let q = p[(i + 1) % p.len()];
        a += p[i][0] * q[1] - q[0] * p[i][1];
    }
    a / 2.0
}

/// Sutherland–Hodgman against one half-plane, keeping `nx * x + ny * y <= d`. An edge that survives
/// keeps its own tag; the edge created along the cut takes `tag`.
pub fn clip_half(
    poly: &[[f64; 2]],
    tags: &[bool],
    nx: f64,
    ny: f64,
    d: f64,
    tag: bool,
) -> (Poly, Vec<bool>) {
    let mut out = Vec::new();
    let mut ot = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let t = tags[i];
        let da = nx * a[0] + ny * a[1] - d;
        let db = nx * b[0] + ny * b[1] - d;
        let ain = da <= 0.0;
        let bin = db <= 0.0;
        if ain {
            out.push(a);
            ot.push(t);
            if !bin {
                let s = da / (da - db);
                out.push([a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s]);
                ot.push(tag);
            }
        } else if bin {
            let s = da / (da - db);
            out.push([a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s]);
            ot.push(t);
        }
    }
    (out, ot)
}

/// Clips `poly` against every edge of a counter-clockwise convex `hull`, each cut taking that
/// hull edge's tag.
pub fn clip_to_hull(
    poly: &[[f64; 2]],
    tags: &[bool],
    hull: &[[f64; 2]],
    hull_tags: &[bool],
) -> (Poly, Vec<bool>) {
    let mut p = poly.to_vec();
    let mut t = tags.to_vec();
    for i in 0..hull.len() {
        if p.is_empty() {
            break;
        }
        let a = hull[i];
        let b = hull[(i + 1) % hull.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let l = hypot2(dx, dy);
        if l < 1e-12 {
            continue;
        }
        // Outward normal of a counter-clockwise hull.
        let nx = dy / l;
        let ny = -dx / l;
        let (np, nt) = clip_half(&p, &t, nx, ny, nx * a[0] + ny * a[1], hull_tags[i]);
        p = np;
        t = nt;
    }
    (p, t)
}

/// Pulls every edge in by `w`. For a convex polygon, clipping by its own edges moved inward *is*
/// the offset, and it collapses to nothing rather than self-intersecting when `w` grows too big for
/// the shape.
pub fn trim_convex(poly: &[[f64; 2]], w: f64) -> Poly {
    let mut p = poly.to_vec();
    let mut t = vec![true; poly.len()];
    for i in 0..poly.len() {
        if p.is_empty() {
            break;
        }
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let l = hypot2(dx, dy);
        if l < 1e-12 {
            continue;
        }
        let nx = dy / l;
        let ny = -dx / l;
        let (np, nt) = clip_half(&p, &t, nx, ny, nx * a[0] + ny * a[1] - w, true);
        p = np;
        t = nt;
    }
    p
}

/// Drops zero-length edges, keeping tags in step with the vertices.
pub fn dedupe(p: &[[f64; 2]], t: &[bool], eps: f64) -> (Poly, Vec<bool>) {
    let mut out = Vec::new();
    let mut ot = Vec::new();
    for i in 0..p.len() {
        let q = p[i];
        let r = p[(i + 1) % p.len()];
        if hypot2(r[0] - q[0], r[1] - q[1]) < eps {
            continue;
        }
        out.push(q);
        ot.push(t[i]);
    }
    (out, ot)
}

/// Pushes every tagged edge *outward* by `w`, leaves the untagged hex seams exactly where they are,
/// and takes the corners where the moved lines cross.
///
/// Outward is the safe direction: an inset can fold a thin shape inside out and has to be thrown
/// away, an offset never can, so no fragment of a field is ever lost. Because every wall line comes
/// from the parcel polygon — which both sides of a hex seam were cut from — two neighbouring pieces
/// compute the identical corner and their sides line up across the seam. A very sharp corner would
/// throw the mitre a long way out, so the move is capped at `cap`.
pub fn mitre_offset(poly: &[[f64; 2]], tags: &[bool], w: f64, cap: f64) -> Poly {
    let n = poly.len();
    let mut lines: Vec<Option<(f64, f64, f64)>> = Vec::with_capacity(n);
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = hypot2(dx, dy);
        if len < 1e-12 {
            lines.push(None);
            continue;
        }
        let nx = dy / len;
        let ny = -dx / len;
        lines.push(Some((
            nx,
            ny,
            nx * a[0] + ny * a[1] + if tags[i] { w } else { 0.0 },
        )));
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = lines[(i + n - 1) % n];
        let b = lines[i];
        let v = poly[i];
        let q = match (a, b) {
            (Some(a), Some(b)) => {
                let det = a.0 * b.1 - a.1 * b.0;
                if det.abs() > 1e-9 {
                    Some([(a.2 * b.1 - b.2 * a.1) / det, (a.0 * b.2 - b.0 * a.2) / det])
                } else {
                    None
                }
            }
            _ => None,
        };
        let Some(q) = q else {
            out.push([v[0], v[1]]);
            continue;
        };
        let ex = q[0] - v[0];
        let ey = q[1] - v[1];
        let d = hypot2(ex, ey);
        out.push(if d > cap {
            [v[0] + ex * cap / d, v[1] + ey * cap / d]
        } else {
            q
        });
    }
    out
}

/// Inradius of a convex polygon, near enough: area over half-perimeter. Used to spot pieces that are
/// scratches rather than fields.
pub fn poly_thickness(p: &[[f64; 2]]) -> f64 {
    let mut per = 0.0;
    for i in 0..p.len() {
        let q = p[i];
        let r = p[(i + 1) % p.len()];
        per += hypot2(r[0] - q[0], r[1] - q[1]);
    }
    if per < 1e-12 {
        0.0
    } else {
        2.0 * poly_area(p) / per
    }
}

/// Halves a zone's bounding box again and again with straight cuts, always across whatever axis is
/// currently widest, at a wobbly middle and with the cut angle nudged a few degrees so nothing lines
/// up into a grid.
///
/// A Voronoi split would tile the plane with no room left over; this leaves the margin the grass
/// strips between fields are shaved out of.
pub fn parcel_split(
    poly: &[[f64; 2]],
    rnd: &mut Mulberry32,
    max_w: f64,
    min_w: f64,
    out: &mut Vec<Poly>,
    depth: u32,
) {
    if depth > 13 || poly.len() < 3 {
        out.push(poly.to_vec());
        return;
    }
    let mut best: Option<([f64; 2], f64)> = None;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let l = hypot2(dx, dy);
        if l < 1e-12 {
            continue;
        }
        for d in [[dx / l, dy / l], [-dy / l, dx / l]] {
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for q in poly {
                let s = q[0] * d[0] + q[1] * d[1];
                if s < lo {
                    lo = s;
                }
                if s > hi {
                    hi = s;
                }
            }
            if best.is_none_or(|b| hi - lo > b.1) {
                best = Some((d, hi - lo));
            }
        }
    }
    let Some((axis, span)) = best else {
        out.push(poly.to_vec());
        return;
    };
    if span <= max_w || span < min_w * 2.0 {
        out.push(poly.to_vec());
        return;
    }

    let a = libm::atan2(axis[1], axis[0]) + (rnd.next_f64() - 0.5) * 0.34;
    let nx = cos(a);
    let ny = sin(a);
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for q in poly {
        let s = q[0] * nx + q[1] * ny;
        if s < lo {
            lo = s;
        }
        if s > hi {
            hi = s;
        }
    }
    let span = hi - lo;
    let room = min_w / span;
    let f = (1.0 - room).min(room.max(0.34 + rnd.next_f64() * 0.32));
    let d = lo + span * f;
    let tags = vec![true; poly.len()];
    let (pa, _) = clip_half(poly, &tags, nx, ny, d, true);
    let (pb, _) = clip_half(poly, &tags, -nx, -ny, -d, true);
    if pa.len() < 3 || pb.len() < 3 {
        out.push(poly.to_vec());
        return;
    }
    parcel_split(&pa, rnd, max_w, min_w, out, depth + 1);
    parcel_split(&pb, rnd, max_w, min_w, out, depth + 1);
}

/// Which of `polys` contains `q`? They are convex and counter-clockwise, so inside means left of
/// every edge.
pub fn hull_at(polys: &[Poly], q: [f64; 2]) -> Option<usize> {
    for (i, h) in polys.iter().enumerate() {
        let mut hit = true;
        for k in 0..h.len() {
            let a = h[k];
            let b = h[(k + 1) % h.len()];
            if (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]) < -1e-12 {
                hit = false;
                break;
            }
        }
        if hit {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    fn square(r: f64) -> Poly {
        vec![[-r, -r], [r, -r], [r, r], [-r, r]]
    }

    #[test]
    fn area_is_signed_and_thickness_is_the_inradius() {
        let s = square(1.0);
        assert_eq!(poly_area(&s), 4.0);
        let mut flipped = s.clone();
        flipped.reverse();
        assert_eq!(poly_area(&flipped), -4.0, "winding flips the sign");
        // A 2x2 square: area 4, perimeter 8, so 2 * 4 / 8 = 1, its inradius exactly.
        assert_eq!(poly_thickness(&s), 1.0);
        assert_eq!(poly_thickness(&[]), 0.0);
    }

    #[test]
    fn clipping_keeps_the_kept_side_and_tags_the_new_edge() {
        let s = square(1.0);
        let tags = vec![false; 4];
        // Keep x <= 0: the right half goes.
        let (p, t) = clip_half(&s, &tags, 1.0, 0.0, 0.0, true);
        assert_eq!(poly_area(&p), 2.0, "half the square survives");
        assert!(p.iter().all(|q| q[0] <= 1e-12));
        assert_eq!(t.iter().filter(|&&x| x).count(), 1, "one new edge, tagged");

        // A cut that misses keeps everything, and one that misses the other way keeps nothing.
        let (all, _) = clip_half(&s, &tags, 1.0, 0.0, 5.0, true);
        assert_eq!(all.len(), 4);
        let (none, _) = clip_half(&s, &tags, 1.0, 0.0, -5.0, true);
        assert!(none.is_empty());
    }

    #[test]
    fn clipping_to_a_hull_intersects_with_it() {
        let big = square(2.0);
        let small = square(1.0);
        let (p, t) = clip_to_hull(&big, &[true; 4], &small, &[false; 4]);
        assert!(
            (poly_area(&p) - 4.0).abs() < 1e-12,
            "the intersection is the small square"
        );
        assert!(t.iter().all(|&x| !x), "every edge came from the hull");
    }

    #[test]
    fn trimming_shrinks_a_convex_shape_and_then_collapses_it() {
        let s = square(1.0);
        let t = trim_convex(&s, 0.25);
        assert!((poly_area(&t) - 2.25).abs() < 1e-12, "a 1.5 x 1.5 square");
        assert!(
            trim_convex(&s, 2.0).len() < 3,
            "trimming past the middle collapses rather than folding inside out"
        );
    }

    #[test]
    fn dedupe_drops_zero_length_edges_and_keeps_tags_aligned() {
        let p = vec![[0.0, 0.0], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let t = vec![true, false, true, false];
        let (q, tq) = dedupe(&p, &t, 1e-9);
        assert_eq!(q.len(), 3);
        assert_eq!(q[0], [0.0, 0.0]);
        assert_eq!(
            tq,
            vec![false, true, false],
            "the dropped vertex takes its own tag with it; the survivor keeps its own"
        );
    }

    #[test]
    fn the_mitre_moves_tagged_edges_out_and_leaves_seams_alone() {
        let s = square(1.0);
        // Every edge tagged: the square grows by w on all four sides.
        let all = mitre_offset(&s, &[true; 4], 0.5, 10.0);
        assert!((poly_area(&all) - 9.0).abs() < 1e-9, "a 3 x 3 square");

        // The bottom edge untagged: that side stays put, so the shape grows only on three.
        let some = mitre_offset(&s, &[false, true, true, true], 0.5, 10.0);
        assert!(
            (some[0][1] - -1.0).abs() < 1e-9,
            "the seam's own y is unmoved"
        );
        assert!((some[1][1] - -1.0).abs() < 1e-9);
        assert!((some[2][1] - 1.5).abs() < 1e-9, "the far side moved out");
    }

    #[test]
    fn the_mitre_cap_bounds_how_far_a_sharp_corner_flies() {
        // A sliver: the corner at the origin is very sharp, so the mitre wants to fly far out.
        let sliver = vec![[0.0, 0.0], [10.0, 0.02], [10.0, -0.02]];
        let capped = mitre_offset(&sliver, &[true; 3], 0.5, 1.0);
        let flown = hypot2(capped[0][0] - 0.0, capped[0][1] - 0.0);
        assert!(
            flown <= 1.0 + 1e-9,
            "the move is capped at 1.0, went {flown}"
        );
        assert!(flown > 0.9, "and it does travel most of the way");
    }

    #[test]
    fn a_parcel_split_covers_the_original_and_respects_the_widths() {
        let box_ = square(50.0);
        let mut rnd = Mulberry32::from_i32(12345);
        let mut out = Vec::new();
        parcel_split(&box_, &mut rnd, 34.0, 18.0, &mut out, 0);
        assert!(out.len() > 1, "a 100-wide box splits");
        let total: f64 = out.iter().map(|p| poly_area(p)).sum();
        assert!(
            (total - poly_area(&box_)).abs() < 1e-6,
            "the pieces tile the box exactly: {total} vs {}",
            poly_area(&box_)
        );
        for p in &out {
            assert!(p.len() >= 3);
        }

        // Below max_w nothing is cut.
        let small = square(10.0);
        let mut out2 = Vec::new();
        parcel_split(&small, &mut rnd, 34.0, 18.0, &mut out2, 0);
        assert_eq!(out2.len(), 1);
    }

    #[test]
    fn hull_at_finds_the_polygon_a_point_sits_in() {
        let left = vec![[-2.0, -1.0], [0.0, -1.0], [0.0, 1.0], [-2.0, 1.0]];
        let right = vec![[0.0, -1.0], [2.0, -1.0], [2.0, 1.0], [0.0, 1.0]];
        let polys = vec![left, right];
        assert_eq!(hull_at(&polys, [-1.0, 0.0]), Some(0));
        assert_eq!(hull_at(&polys, [1.0, 0.0]), Some(1));
        assert_eq!(hull_at(&polys, [9.0, 0.0]), None);
        assert_eq!(
            hull_at(&polys, [0.0, 0.0]),
            Some(0),
            "a shared edge picks the first"
        );
    }
}
