//! Villages, prototype section 4d (`buildHouses`): zoned like fields and a wood, but laid out with
//! order rather than scatter.
//!
//! Plots come off a street grid whose pitch is a little wider than a house, so buildings stand
//! apart with yards between them, and every empty plot reads as a street or a garden. Each house
//! picks one of four quarter turns plus a few degrees of play: a village holds ridges running
//! several ways, which keeps a regular grid from reading as graph paper, while the shared wall tone
//! per zone keeps it reading as one settlement.
//!
//! A building is walls plus a 45-degree gable that overhangs the wall and carries a fascia at the
//! eave, so the roof reads as a slab with thickness instead of a sheet folded over a box. Doors and
//! windows are decal quads floated a hair off the wall — cheaper than cutting the wall into panels,
//! and indistinguishable at this scale.

use cl_hexsphere::{Frames, HexSphere, facet_plane};
use cl_model::{Cover, MeshData, WorldSnapshot, hex_rgb};
use cl_noise::vec::{V3, add, cross, dot, len, mul, norm, sub};
use cl_noise::{hash3i, js};

use crate::poly::{Poly, hull_at, poly_area};
use crate::zones::{cover_zones, ring_normal, zone_frame};

/// Wall tones. One is drawn per village, so a settlement is built of one thing.
pub const HOUSE_WALLS: [&str; 4] = ["#e7ddc8", "#dcd0b8", "#cfc2a8", "#e2d5bd"];
/// The one roof: brick red, with its own shadow tone for courses and the ridge cap.
pub const HOUSE_ROOF: [&str; 2] = ["#a24b32", "#823a26"];
/// Doors and windows, both the same single colour.
pub const HOUSE_OPEN: &str = "#453227";
/// Street grid pitch, in world pixels.
pub const PLOT_PX: f64 = 9.0;
/// How far a plot centre wanders off its slot, in world pixels.
pub const PLOT_JIT: f64 = 1.0;
/// Narrowest house across the ridge, in world pixels.
pub const HOUSE_SPAN_MIN: f64 = 4.0;
/// Widest house across the ridge. The range is narrow on purpose: the gable peak sits at wall
/// height plus half the span, so anything that widens a house also makes it taller, and a village
/// with an even roofline needs both held close.
pub const HOUSE_SPAN_MAX: f64 = 5.0;
/// Shortest house along the ridge, in whole world pixels.
pub const HOUSE_LEN_MIN: f64 = 4.0;
/// Longest house along the ridge, inclusive. Length carries the variety, and it is rolled in whole
/// world pixels so neighbours come out 1, 2, 3 or 4 px apart rather than 2.7.
pub const HOUSE_LEN_MAX: f64 = 8.0;
/// Lowest wall, in world pixels.
pub const WALL_MIN: f64 = 3.2;
/// Highest wall, in world pixels.
pub const WALL_MAX: f64 = 4.2;
/// Eave overhang past the wall, in world pixels.
pub const ROOF_OVER: f64 = 1.0;
/// Roof slab thickness, in world pixels, measured perpendicular to the pitch. Offsetting the top
/// surface vertically by this instead leaves only `LIP * cos 45` of visible edge, which is why the
/// roof read as thinner than a pixel.
pub const ROOF_LIP: f64 = 1.0;
/// One course of the roof, in world pixels, measured along the ridge.
pub const STRIPE_PX: f64 = 2.0;
/// The single band capping the ridge, in world pixels.
pub const RIDGE_CAP_PX: f64 = 1.5;
/// Share of houses whose courses alternate tone. Decided per house, so a village mixes both.
pub const STRIPE_ODDS: f64 = 0.55;
/// Wall-coloured trim down each gable edge, in world pixels.
pub const VERGE_PX: f64 = 1.0;
/// Share of roofs carrying that trim.
pub const VERGE_ODDS: f64 = 0.20;
/// Door width, in world pixels: about a third of the facade at this scale.
pub const DOOR_W: f64 = 2.0;
/// Door height, in world pixels.
pub const DOOR_H: f64 = 2.0;
/// Window size, in world pixels. One pixel is the right proportion for a building this small.
pub const WIN_PX: f64 = 1.0;
/// Chimney thickness, in world pixels.
pub const CHIM_PX: f64 = 1.5;
/// Shortest chimney rise, in world pixels. Rise is what shows above the roof surface at the
/// chimney's own position, not above the ridge: off the ridge those differ by nearly half the span,
/// which is why chimneys out on a pitch came out long.
pub const CHIM_RISE_MIN: f64 = 1.0;
/// Tallest chimney rise, in world pixels.
pub const CHIM_RISE_MAX: f64 = 2.5;
/// Share of houses with a chimney at all.
pub const CHIM_ODDS: f64 = 0.80;
/// Share of houses that grow a wing into an L.
pub const L_CHANCE: f64 = 0.18;
/// How deep a house is buried, in world pixels, so none hovers over its facet.
pub const HOUSE_SINK: f64 = 0.6;
/// Share of plots that carry a house; the rest become yards and streets.
pub const HOUSE_ODDS: f64 = 0.85;
/// Half-angle cap of a village zone, in radians.
pub const HOUSE_SPAN: f64 = 0.50;
/// Salt of the village zone frames.
pub const HOUSE_SALT: i32 = 71;
/// The quarter turns a house may take. Each house picks its own, so a village holds ridges running
/// four ways rather than one.
pub const HOUSE_TURNS: [f64; 4] = [
    0.0,
    std::f64::consts::FRAC_PI_4,
    std::f64::consts::FRAC_PI_2,
    3.0 * std::f64::consts::FRAC_PI_4,
];
/// Play on top of that turn, in radians, so the rows do not look machined.
pub const HOUSE_ROT_JIT: f64 = 0.06;

/// How far a decal floats off its wall, as a share of one world pixel.
const DEC: f64 = 0.12;

/// The villages of one world: every house and wing in a single mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct Houses {
    /// Walls, roofs, openings and chimneys, flat-shaded with vertex colours and no texture.
    pub surface: MeshData,
    /// Connected zones the villages were laid out in.
    pub zones: usize,
    /// Tiles carrying town cover.
    pub tiles: usize,
    /// Houses built.
    pub houses: usize,
    /// Wings added to them.
    pub wings: usize,
}

/// One building's placement, size, tone and rolls.
struct Building {
    /// Ground anchor, already sunk by [`HOUSE_SINK`].
    g: V3,
    /// Across the ridge.
    ax: V3,
    /// Along the ridge.
    az: V3,
    /// Away from the planet.
    up: V3,
    /// Span across the ridge.
    w: f64,
    /// Length along the ridge.
    d: f64,
    /// Wall height.
    wt: f64,
    /// This village's wall tone.
    wall: [f64; 3],
    /// Whether the courses alternate tone.
    striped: bool,
    /// Whether the gable edges carry trim.
    verge: bool,
    /// Hash key of the plot, and which side the door faces.
    kx: i32,
    ky: i32,
    kz: i32,
    front: usize,
}

/// The tones shared by every house: the two roof shades and the one opening colour.
struct Tones {
    roof_a: [f64; 3],
    roof_b: [f64; 3],
    open: [f64; 3],
}

/// Emits faces into one mesh at a fixed world-pixel scale.
struct Walls<'a> {
    mesh: &'a mut MeshData,
    px: f64,
}

/// The prototype's `buildHouses`. `None` where the world carries no town at all.
///
/// `px` is one world pixel, `frames.px` in practice; `seed` is the world seed.
///
/// # Panics
/// If `snapshot` and `sphere` disagree on the tile count.
pub fn build_houses(
    sphere: &HexSphere,
    frames: &Frames,
    snapshot: &WorldSnapshot,
    px: f64,
    seed: f64,
) -> Option<Houses> {
    assert_eq!(
        snapshot.tiles.len(),
        sphere.len(),
        "one tile state per tile"
    );
    assert_eq!(frames.tiles.len(), sphere.len(), "one frame per tile");
    let zoning = cover_zones(sphere, snapshot, Cover::Town, HOUSE_SPAN);
    if zoning.total == 0 {
        return None;
    }
    let tone = |h: &str| {
        let c = hex_rgb(h);
        [
            f64::from(c[0]) / 255.0,
            f64::from(c[1]) / 255.0,
            f64::from(c[2]) / 255.0,
        ]
    };
    let wall_set = HOUSE_WALLS.map(tone);
    let tones = Tones {
        roof_a: tone(HOUSE_ROOF[0]),
        roof_b: tone(HOUSE_ROOF[1]),
        open: tone(HOUSE_OPEN),
    };
    let seed_i = js::to_int32(seed);

    let mut mesh = MeshData::default();
    let mut houses = 0;
    let mut wings = 0;

    for (zi, zone) in zoning.zones.iter().enumerate() {
        let frame = zone_frame(sphere, zone, HOUSE_SALT);
        let kz = seed_i.wrapping_add((zi as i32).wrapping_mul(617));
        // One wall tone per village, so a settlement is built of one thing.
        let wall = wall_set[(hash3i(zone[0].0 as i32, zone.len() as i32, kz.wrapping_add(3))
            * wall_set.len() as f64
            * 0.999) as usize];

        // Tile outlines in the zone's plane, counter-clockwise so `hull_at` can test them.
        let hulls: Vec<Poly> = zone
            .iter()
            .map(|&id| {
                let mut h: Poly = sphere
                    .tile(id)
                    .corners
                    .iter()
                    .map(|&p| frame.to2e(p))
                    .collect();
                if poly_area(&h) < 0.0 {
                    h.reverse();
                }
                h
            })
            .collect();
        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for h in &hulls {
            for r in h {
                x0 = x0.min(r[0]);
                x1 = x1.max(r[0]);
                y0 = y0.min(r[1]);
                y1 = y1.max(r[1]);
            }
        }
        let step = PLOT_PX * px;
        let gi0 = (x0 / step).floor() as i32 - 1;
        let gi1 = (x1 / step).ceil() as i32 + 1;
        let gj0 = (y0 / step).floor() as i32 - 1;
        let gj1 = (y1 / step).ceil() as i32 + 1;

        for sj in gj0..=gj1 {
            for si in gi0..=gi1 {
                let q = [
                    (f64::from(si)
                        + 0.5
                        + (hash3i(si, sj, kz.wrapping_add(3)) - 0.5) * PLOT_JIT / PLOT_PX)
                        * step,
                    (f64::from(sj)
                        + 0.5
                        + (hash3i(si, sj, kz.wrapping_add(9)) - 0.5) * PLOT_JIT / PLOT_PX)
                        * step,
                ];
                let Some(hi) = hull_at(&hulls, q) else {
                    continue;
                };
                if hash3i(si, sj, kz.wrapping_add(17)) > HOUSE_ODDS {
                    continue; // an empty plot: a yard or a stretch of street
                }
                let tile = zone[hi];

                let t = sphere.tile(tile);
                let tf = &frames.tiles[tile.index()];
                let plane = facet_plane(t, tf);
                let dir = frame.to3e(q);
                let mut g = mul(dir, plane.d * tf.radius / dot(dir, plane.n));
                let up = norm(g);
                let mut a1 = sub(frame.e1, mul(up, dot(frame.e1, up)));
                if len(a1) < 1e-6 {
                    a1 = sub(frame.e2, mul(up, dot(frame.e2, up)));
                }
                let a1 = norm(a1);
                let rot = HOUSE_TURNS[(hash3i(si, sj, kz.wrapping_add(71)) * 3.999) as usize]
                    + (hash3i(si, sj, kz.wrapping_add(79)) - 0.5) * 2.0 * HOUSE_ROT_JIT;
                let ax = add(mul(a1, js::cos(rot)), mul(cross(up, a1), js::sin(rot)));
                let az = cross(up, ax);
                g = sub(g, mul(up, HOUSE_SINK * px));

                let w = (HOUSE_SPAN_MIN
                    + (HOUSE_SPAN_MAX - HOUSE_SPAN_MIN) * hash3i(si, sj, kz.wrapping_add(23)))
                    * px;
                let steps = HOUSE_LEN_MAX - HOUSE_LEN_MIN + 1.0;
                let d = (HOUSE_LEN_MIN
                    + (hash3i(si, sj, kz.wrapping_add(31)) * steps * 0.999).floor())
                    * px;
                let wt =
                    (WALL_MIN + (WALL_MAX - WALL_MIN) * hash3i(si, sj, kz.wrapping_add(37))) * px;
                let front = (hash3i(si, sj, kz.wrapping_add(61)) * 3.999) as usize;
                let striped = hash3i(si, sj, kz.wrapping_add(13)) < STRIPE_ODDS;
                let verge = hash3i(si, sj, kz.wrapping_add(19)) < VERGE_ODDS;

                let mut walls = Walls {
                    mesh: &mut mesh,
                    px,
                };
                walls.building(
                    &Building {
                        g,
                        ax,
                        az,
                        up,
                        w,
                        d,
                        wt,
                        wall,
                        striped,
                        verge,
                        kx: si,
                        ky: sj,
                        kz,
                        front,
                    },
                    &tones,
                );
                houses += 1;

                // A wing, set at right angles and a touch lower, turns the box into an L. It shares
                // walls and roof so the two read as one building, and it keeps the main span so the
                // two roofs sit at the same height.
                if hash3i(si, sj, kz.wrapping_add(67)) < L_CHANCE {
                    let w2 = w;
                    let d2 = (3.0 + (hash3i(si, sj, kz.wrapping_add(83)) * 2.999).floor()) * px;
                    let off = add(
                        mul(ax, (w / 2.0 - d2 / 2.0) * 0.9),
                        mul(az, (d / 2.0 + w2 / 2.0) - w2 * 0.35),
                    );
                    walls.building(
                        &Building {
                            g: add(g, off),
                            ax: az,
                            az: mul(ax, -1.0),
                            up,
                            w: w2,
                            d: d2,
                            wt: wt * 0.95,
                            wall,
                            striped,
                            verge,
                            kx: si.wrapping_add(911),
                            ky: sj,
                            kz,
                            front: 1,
                        },
                        &tones,
                    );
                    wings += 1;
                }
            }
        }
    }

    if mesh.is_empty() {
        return None;
    }
    Some(Houses {
        surface: mesh,
        zones: zoning.zones.len(),
        tiles: zoning.total,
        houses,
        wings,
    })
}

impl Walls<'_> {
    /// Appends one flat polygon as a fan, wound so it faces `out`.
    ///
    /// Winding is fixed against a supplied outward direction rather than tracked by hand: a house
    /// has thirty-odd faces pointing six ways, and deriving each one's winding from its normal is
    /// what keeps them all facing out.
    fn face(&mut self, ring: &[V3], c: [f64; 3], out: V3) {
        let n = ring_normal(ring);
        let flip = dot(n, out) < 0.0;
        let n = if flip { mul(n, -1.0) } else { n };
        let at = |i: usize| ring[if flip { ring.len() - 1 - i } else { i }];
        for i in 1..ring.len() - 1 {
            for p in [at(0), at(i), at(i + 1)] {
                self.mesh.push_vertex(p, n);
                self.mesh
                    .colors
                    .extend([c[0] as f32, c[1] as f32, c[2] as f32]);
            }
        }
    }

    /// One building: walls, the two roof pitches over them, openings and a chimney.
    ///
    /// The gable end is wall, cut as the classic five-sided house silhouette, and the roof is only
    /// the two pitches laid over it — a caret, not a solid triangular block. The roof underside
    /// meets the wall exactly at the wall top, so the eave overhangs without opening a slot into
    /// the interior.
    fn building(&mut self, b: &Building, tones: &Tones) {
        let px = self.px;
        let p =
            |x: f64, y: f64, z: f64| add(b.g, add(mul(b.ax, x), add(mul(b.up, y), mul(b.az, z))));
        let (hw, hd) = (b.w / 2.0, b.d / 2.0);
        let w = hw + ROOF_OVER * px;
        let d = hd + ROOF_OVER * px;
        let eb = b.wt - ROOF_OVER * px; // eave, underside
        // A 45-degree pitch needs LIP * sqrt2 of vertical offset to end up LIP thick measured
        // square to the slope.
        let et = eb + ROOF_LIP * std::f64::consts::SQRT_2 * px;
        let rt = et + w; // ridge, top
        let rb = eb + w; // ridge, underside
        // The eave is cut square to the pitch, not vertically: a vertical cut ends each arm of the
        // caret in an upright face and blunts the silhouette. Sliding the underside corner in and
        // up by this leaves a visible roof edge exactly ROOF_LIP long in the plane of the cut.
        let mit = ROOF_LIP * px / std::f64::consts::SQRT_2;

        // Long walls.
        for s in [-1.0, 1.0] {
            self.face(
                &[
                    p(s * hw, 0.0, -hd),
                    p(s * hw, 0.0, hd),
                    p(s * hw, b.wt, hd),
                    p(s * hw, b.wt, -hd),
                ],
                b.wall,
                mul(b.ax, s),
            );
        }
        // Gable walls: the peak is wall, not roof.
        for e in [-1.0, 1.0] {
            self.face(
                &[
                    p(-hw, 0.0, e * hd),
                    p(hw, 0.0, e * hd),
                    p(hw, b.wt, e * hd),
                    p(0.0, b.wt + hw, e * hd),
                    p(-hw, b.wt, e * hd),
                ],
                b.wall,
                mul(b.az, e),
            );
        }

        // Courses run along the ridge, so on the slope they read as vertical stripes, and a single
        // band caps the ridge across the top.
        let cap_f = 0.5f64.min(RIDGE_CAP_PX * px / w);
        let vg = if b.verge { VERGE_PX * px } else { 0.0 };
        let z_in = (d * 0.25).max(d - vg);
        let m = 2.0f64.max(js::round(2.0 * z_in / (STRIPE_PX * px)));
        for s in [-1.0, 1.0] {
            let out = norm(add(mul(b.ax, s), b.up));
            let x_e = s * w;
            let x_c = s * w * cap_f;
            let y_c = et + w * (1.0 - cap_f);
            for i in 0..m as usize {
                let z0 = -z_in + 2.0 * z_in * i as f64 / m;
                let z1 = -z_in + 2.0 * z_in * (i + 1) as f64 / m;
                let c = if b.striped && i % 2 == 1 {
                    tones.roof_b
                } else {
                    tones.roof_a
                };
                self.face(
                    &[
                        p(x_e, et, z0),
                        p(x_e, et, z1),
                        p(x_c, y_c, z1),
                        p(x_c, y_c, z0),
                    ],
                    c,
                    out,
                );
            }
            // Wall-coloured trim down each gable edge, eave to ridge.
            if b.verge {
                for e in [-1.0, 1.0] {
                    self.face(
                        &[
                            p(x_e, et, e * z_in),
                            p(x_e, et, e * d),
                            p(x_c, y_c, e * d),
                            p(x_c, y_c, e * z_in),
                        ],
                        b.wall,
                        out,
                    );
                    self.face(
                        &[
                            p(x_c, y_c, e * z_in),
                            p(x_c, y_c, e * d),
                            p(0.0, rt, e * d),
                            p(0.0, rt, e * z_in),
                        ],
                        b.wall,
                        out,
                    );
                }
            }
            self.face(
                &[
                    p(x_c, y_c, -z_in),
                    p(x_c, y_c, z_in),
                    p(0.0, rt, z_in),
                    p(0.0, rt, -z_in),
                ],
                tones.roof_b,
                out,
            ); // ridge cap
            // Underside, and the mitred edge that gives the roof its depth. Where a verge runs, the
            // spans of that edge beneath it take the wall tone: this face is the depth side of the
            // wall-coloured trim, and leaving it roof-red was the join showing through.
            let x_b = s * (w - mit);
            let y_b = eb + mit;
            self.face(
                &[
                    p(x_b, y_b, -d),
                    p(x_b, y_b, d),
                    p(0.0, rb, d),
                    p(0.0, rb, -d),
                ],
                tones.roof_b,
                mul(out, -1.0),
            );
            let edge_out = norm(sub(mul(b.ax, s), b.up));
            let band = |z0: f64, z1: f64, c: [f64; 3]| {
                let ring = [
                    p(s * w, et, z0),
                    p(s * w, et, z1),
                    p(x_b, y_b, z1),
                    p(x_b, y_b, z0),
                ];
                (ring, c)
            };
            if b.verge {
                for (ring, c) in [
                    band(-d, -z_in, b.wall),
                    band(-z_in, z_in, tones.roof_b),
                    band(z_in, d, b.wall),
                ] {
                    self.face(&ring, c, edge_out);
                }
            } else {
                let (ring, c) = band(-d, d, tones.roof_b);
                self.face(&ring, c, edge_out);
            }
        }
        // The gable-end caret. Its outer corner is the mitred one, so each arm ends in a 45-degree
        // point. Wall-coloured when the roof carries a verge, since this whole face is the depth
        // side of that trim.
        for e in [-1.0, 1.0] {
            for s in [-1.0, 1.0] {
                self.face(
                    &[
                        p(s * w, et, e * d),
                        p(0.0, rt, e * d),
                        p(0.0, rb, e * d),
                        p(s * (w - mit), eb + mit, e * d),
                    ],
                    if b.verge { b.wall } else { tones.roof_b },
                    mul(b.az, e),
                );
            }
        }

        self.openings(b, tones, hw, hd);
        self.chimney(b, et, eb, w);
    }

    /// Doors and windows, floated decal off the wall. Every house gets a door on its front;
    /// windows fill whatever wall length is left over.
    fn openings(&mut self, b: &Building, tones: &Tones, hw: f64, hd: f64) {
        let px = self.px;
        let sides = [
            (mul(b.az, -1.0), hd, b.ax, b.w),
            (b.az, hd, b.ax, b.w),
            (mul(b.ax, -1.0), hw, b.az, b.d),
            (b.ax, hw, b.az, b.d),
        ];
        let (fo, foff, fhx, flen) = sides[b.front];
        let door_u = (hash3i(b.kx, b.ky, b.kz.wrapping_add(11)) - 0.5)
            * 0.0f64.max(flen - (DOOR_W + 1.0) * px);
        self.panel(
            b,
            fo,
            foff,
            fhx,
            door_u,
            0.0,
            DOOR_W * px,
            DOOR_H * px,
            tones.open,
        );
        for (si, &(o, off, hx, slen)) in sides.iter().enumerate() {
            let n = (slen / (2.6 * px)).floor().clamp(1.0, 3.0);
            for i in 0..n as usize {
                let u = ((i as f64 + 0.5) / n - 0.5) * slen * 0.78;
                if si == b.front && (u - door_u).abs() < (DOOR_W + WIN_PX) * 0.6 * px {
                    continue; // the door already has this stretch of wall
                }
                self.panel(
                    b,
                    o,
                    off,
                    hx,
                    u,
                    b.wt * 0.45,
                    WIN_PX * px,
                    WIN_PX * px,
                    tones.open,
                );
            }
        }
    }

    /// One opening quad, floated off the wall it sits on.
    #[allow(clippy::too_many_arguments)]
    fn panel(
        &mut self,
        b: &Building,
        o: V3,
        off: f64,
        hx: V3,
        u: f64,
        v: f64,
        pw: f64,
        ph: f64,
        c: [f64; 3],
    ) {
        let base = mul(o, off + DEC * self.px);
        let corner = |du: f64, dv: f64| add(b.g, add(base, add(mul(hx, du), mul(b.up, dv))));
        self.face(
            &[
                corner(u - pw / 2.0, v),
                corner(u + pw / 2.0, v),
                corner(u + pw / 2.0, v + ph),
                corner(u - pw / 2.0, v + ph),
            ],
            c,
            o,
        );
    }

    /// A chimney in the wall material, poking through one pitch. A fifth of houses have none.
    fn chimney(&mut self, b: &Building, et: f64, eb: f64, w: f64) {
        if hash3i(b.kx, b.ky, b.kz.wrapping_add(97)) >= CHIM_ODDS {
            return;
        }
        let px = self.px;
        let p =
            |x: f64, y: f64, z: f64| add(b.g, add(mul(b.ax, x), add(mul(b.up, y), mul(b.az, z))));
        let cs = if hash3i(b.kx, b.ky, b.kz.wrapping_add(53)) > 0.5 {
            1.0
        } else {
            -1.0
        };
        let cx = cs * w * 0.45;
        let cz = (hash3i(b.kx, b.ky, b.kz.wrapping_add(59)) - 0.5) * b.d * 0.5;
        let rise = CHIM_RISE_MIN
            + (CHIM_RISE_MAX - CHIM_RISE_MIN) * hash3i(b.kx, b.ky, b.kz.wrapping_add(101));
        let ch = CHIM_PX * px * 0.5;
        // The pitch surface here is et + w - |x|, so `rise` is what is actually seen.
        let ctop = et + w - cx.abs() + rise * px;
        for (ox, oz, o) in [
            (ch, 0.0, b.ax),
            (-ch, 0.0, mul(b.ax, -1.0)),
            (0.0, ch, b.az),
            (0.0, -ch, mul(b.az, -1.0)),
        ] {
            let (a, c) = if ox == 0.0 {
                ([cx - ch, cz + oz], [cx + ch, cz + oz])
            } else {
                ([cx + ox, cz - ch], [cx + ox, cz + ch])
            };
            self.face(
                &[
                    p(a[0], eb, a[1]),
                    p(c[0], eb, c[1]),
                    p(c[0], ctop, c[1]),
                    p(a[0], ctop, a[1]),
                ],
                b.wall,
                o,
            );
        }
        self.face(
            &[
                p(cx - ch, ctop, cz - ch),
                p(cx + ch, ctop, cz - ch),
                p(cx + ch, ctop, cz + ch),
                p(cx - ch, ctop, cz + ch),
            ],
            b.wall,
            b.up,
        );
    }
}
