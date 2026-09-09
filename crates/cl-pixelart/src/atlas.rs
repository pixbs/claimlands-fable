//! The ground atlas: one 24×24 cell per tile, prototype section 3b (`buildTerrainAtlas`).
//!
//! Every texel is mapped back to the direction it covers and a continuous 3D field is sampled
//! there, so the patchwork crosses tile borders unbroken and no two hexes are alike. The field is
//! quantised into value bands and the band edge is dithered with a world-space hash, which is what
//! produces the stippled boundaries rather than hard contours.

use cl_hexsphere::{Frames, HexSphere, facet_plane, texel_dir};
use cl_model::world::{TILE_PX, UV_INSET};
use cl_model::{Filter, RgbaImage, Texture, TileId, WorldSnapshot, Wrap, hex_rgb};
use cl_noise::js::{cos, hypot2, pow, sin};
use cl_noise::vec::dot;
use cl_noise::{fbm3, hash3};

use crate::palette::{GRASS_BANDS, MUD_BANDS, SEA_BANDS};

/// Octaves of the grass field; with [`GRASS_F0`] the patches come out about two tiles across.
pub const GRASS_OCT: u32 = 4;
/// Base frequency of the grass field.
pub const GRASS_F0: f64 = 5.5;
/// Octaves of the sea field.
pub const SEA_OCT: u32 = 2;
/// Base frequency of the sea field.
pub const SEA_F0: f64 = 3.0;
/// Per-texel jitter of the grass band edge; 0 would give vector-clean contours.
pub const GRASS_DITHER: f64 = 0.025;
/// The same on the sea, where the bands are wider and the edge takes less noise. A literal in the
/// prototype rather than a named constant, so `fixtures/constants.json` does not carry it.
pub const SEA_DITHER: f64 = 0.03;
/// Share of land texels lifted one band: lone flecks catching the light.
pub const SPECKLE: f64 = 0.04;
/// How far the darkest band is driven onto the shoreline.
pub const COAST_DARKEN: f64 = 0.80;
/// Exponent of the coast falloff. Higher keeps the dark tone a rim hugging the water rather than a
/// wash over every tile near it.
pub const COAST_TIGHT: f64 = 2.2;
/// Tiles the sea shelf spreads over, so it reads as shelf giving way to deep ocean.
pub const SEA_FADE: u32 = 3;
/// How far the shelf lifts the sea bands where they meet land.
pub const SEA_SHALLOW: f64 = 0.60;
/// Octaves of the bare-earth field.
pub const MUD_OCT: u32 = 3;
/// Base frequency of the bare-earth field: a few world pixels, well under a tile, because this is a
/// fringe and not a biome.
pub const MUD_F0: f64 = 30.0;
/// Seed offset of the bare-earth field: its own stream, uncorrelated with the grass.
pub const MUD_SALT: f64 = 8681.0;
/// How far the earth reaches before noise decides.
pub const MUD_EDGE: f64 = 0.42;
/// How hard the noise pushes that line about.
pub const MUD_SCATTER: f64 = 0.34;
/// Per-texel speckle on the earth edge, as the grass bands get.
pub const MUD_DITHER: f64 = 0.06;
/// Pitch of the dither hash lattice in world pixels: slightly finer than a texel. Keying it on cell
/// coordinates instead would restart the stipple in every hex. The prototype writes it inline as
/// `K = 1.35 / PX`, so `fixtures/constants.json` does not carry it either.
pub const DITHER_LATTICE: f64 = 1.35;

/// One edge of the cell polygon in cell-local texels: an outward unit normal and its offset, so
/// `d - (nx * x + ny * y)` is positive inside and grows toward the centre.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Edge {
    nx: f64,
    ny: f64,
    d: f64,
}

/// The prototype's `polyEdges`.
fn poly_edges(sides: usize, r: f64) -> Vec<Edge> {
    let pts: Vec<[f64; 2]> = (0..sides)
        .map(|k| {
            let a = k as f64 / sides as f64 * std::f64::consts::PI * 2.0;
            [cos(a) * r, sin(a) * r]
        })
        .collect();
    (0..sides)
        .map(|k| {
            let p = pts[k];
            let q = pts[(k + 1) % sides];
            let nx = q[1] - p[1];
            let ny = -(q[0] - p[0]);
            let l = hypot2(nx, ny);
            let l = if l == 0.0 { 1.0 } else { l };
            Edge {
                nx: nx / l,
                ny: ny / l,
                d: (nx / l) * p[0] + (ny / l) * p[1],
            }
        })
        .collect()
}

/// The window a noise field is stretched over, taken from its own spread across the tile centres.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Range {
    lo: f64,
    span: f64,
}

/// The prototype's `probe`: the 3rd and 97th percentile of the field over all tile centres.
fn probe(sphere: &HexSphere, seed: f64, oct: u32, f0: f64) -> Range {
    let mut a: Vec<f64> = sphere
        .tiles
        .iter()
        .map(|t| fbm3(t.center, seed, oct, f0))
        .collect();
    a.sort_by(|x, y| x.partial_cmp(y).expect("fbm3 is finite"));
    let count = sphere.len() as f64;
    let lo = a[(count * 0.03).floor() as usize];
    let hi = a[(count * 0.97).floor() as usize];
    Range {
        lo,
        span: (hi - lo).max(1e-6),
    }
}

/// The coast, shelf and built-up fields.
///
/// Each is held per tile centre and per corner. A corner's value is a property of the corner
/// itself, so both sides of an edge read the same number and the shading cannot step at a tile
/// border. Tile centres are 0 for the land rim, so it fades out within the coastal tile instead of
/// flooding it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoastFields {
    /// 1 on water, 0 on land: binary, so the dark tone hugs the waterline and stops.
    pub prox: Vec<f64>,
    /// 1 on land, falling to 0 over [`SEA_FADE`] tiles of graph distance.
    pub shallow: Vec<f64>,
    /// Per corner: 1 where any of the three tiles meeting there is water.
    pub corner_prox: Vec<Vec<f64>>,
    /// Per corner: the mean of the three tiles' [`CoastFields::shallow`]. Mean, not max, because
    /// the sea wants a gradient where the land wanted a hard rim.
    pub corner_shallow: Vec<Vec<f64>>,
    /// 1 on a tile carrying a village, 0 elsewhere.
    pub built: Vec<f64>,
    /// Per corner: the share of the three tiles meeting there that carry a village. A corner shared
    /// with a village scores above zero, so the bare ground spills a little way into whatever the
    /// village backs onto, which is what a walked fringe does.
    pub corner_built: Vec<Vec<f64>>,
}

/// `true` where the prototype's tile would read `cover === 'houses'`. A capital is a village with a
/// treasury, so it walks the ground bare the same way; the prototype has no capital to disagree.
fn is_built(snapshot: &WorldSnapshot, id: usize) -> f64 {
    f64::from(u8::from(snapshot.tiles[id].cover.is_building()))
}

/// The prototype's `computeBuilt`.
fn compute_built(sphere: &HexSphere, snapshot: &WorldSnapshot, fields: &mut CoastFields) {
    fields.built = (0..sphere.len()).map(|i| is_built(snapshot, i)).collect();
    fields.corner_built = sphere
        .tiles
        .iter()
        .map(|t| {
            t.corner_tiles
                .iter()
                .map(|c| {
                    (is_built(snapshot, c[0].index())
                        + is_built(snapshot, c[1].index())
                        + is_built(snapshot, c[2].index()))
                        / 3.0
                })
                .collect()
        })
        .collect();
}

/// The prototype's `computeProx`, which ends by calling `computeBuilt`.
fn compute_fields(sphere: &HexSphere, snapshot: &WorldSnapshot) -> CoastFields {
    let count = sphere.len();
    let sea: Vec<bool> = snapshot
        .tiles
        .iter()
        .map(|t| t.terrain.level() < 0)
        .collect();

    // Land rim: binary, so the dark tone hugs the waterline and stops.
    let prox: Vec<f64> = sea.iter().map(|&s| f64::from(u8::from(s))).collect();
    let corner_prox: Vec<Vec<f64>> = sphere
        .tiles
        .iter()
        .map(|t| {
            t.corner_tiles
                .iter()
                .map(|c| {
                    f64::from(u8::from(
                        sea[c[0].index()] || sea[c[1].index()] || sea[c[2].index()],
                    ))
                })
                .collect()
        })
        .collect();

    // Sea shelf: graph distance from land, spread over SEA_FADE tiles.
    let mut dist = vec![-1i32; count];
    let mut queue: Vec<usize> = Vec::with_capacity(count);
    for i in 0..count {
        if !sea[i] {
            dist[i] = 0;
            queue.push(i);
        }
    }
    let mut head = 0;
    while head < queue.len() {
        let i = queue[head];
        head += 1;
        if dist[i] > SEA_FADE as i32 + 1 {
            continue;
        }
        for &j in &sphere.tiles[i].neighbors {
            let j = j.index();
            if dist[j] < 0 {
                dist[j] = dist[i] + 1;
                queue.push(j);
            }
        }
    }
    let shallow: Vec<f64> = (0..count)
        .map(|i| {
            if !sea[i] {
                1.0
            } else if dist[i] < 0 {
                0.0
            } else {
                (1.0 - f64::from(dist[i]) / f64::from(SEA_FADE)).max(0.0)
            }
        })
        .collect();
    let corner_shallow: Vec<Vec<f64>> = sphere
        .tiles
        .iter()
        .map(|t| {
            t.corner_tiles
                .iter()
                .map(|c| {
                    (shallow[c[0].index()] + shallow[c[1].index()] + shallow[c[2].index()]) / 3.0
                })
                .collect()
        })
        .collect();

    let mut fields = CoastFields {
        prox,
        shallow,
        corner_prox,
        corner_shallow,
        built: Vec::new(),
        corner_built: Vec::new(),
    };
    compute_built(sphere, snapshot, &mut fields);
    fields
}

/// Everything a repaint needs that depends only on the sphere and the seed: the band colours, the
/// three field windows, the dither lattice pitch and the two cell polygons.
#[derive(Debug, Clone, PartialEq)]
struct Style {
    grass: [[u8; 3]; 3],
    mud: [[u8; 3]; 3],
    water: [[u8; 3]; 5],
    gp: Range,
    sp: Range,
    mp: Range,
    seed: f64,
    k: f64,
    pentagon: Vec<Edge>,
    hexagon: Vec<Edge>,
}

/// The ground atlas and the state a partial repaint needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Atlas {
    /// `cols * 24` by `rows * 24` texels, clamped and nearest-sampled. Cells past the last tile
    /// stay transparent, exactly as the prototype's cleared canvas leaves them.
    pub texture: Texture,
    /// Cells per row.
    pub cols: u32,
    /// Rows of cells.
    pub rows: u32,
    /// `[col, row]` of each tile's cell, indexed by tile id.
    pub cells: Vec<[u32; 2]>,
    /// The fields the current texels were painted from.
    pub fields: CoastFields,
    style: Style,
}

/// The prototype's `buildTerrainAtlas`. `frames` must come from the snapshot's own levels.
///
/// # Panics
/// If `snapshot`, `frames` and `sphere` disagree on the tile count.
pub fn build_terrain_atlas(
    sphere: &HexSphere,
    frames: &Frames,
    snapshot: &WorldSnapshot,
    seed: f64,
) -> Atlas {
    let count = sphere.len();
    assert_eq!(snapshot.tiles.len(), count, "one tile state per tile");
    assert_eq!(frames.tiles.len(), count, "one frame per tile");

    let cols = (count as f64).sqrt().ceil() as u32;
    let rows = (count as f64 / f64::from(cols)).ceil() as u32;
    let cells: Vec<[u32; 2]> = (0..count as u32).map(|i| [i % cols, i / cols]).collect();

    let bands = |hex: &[&str]| -> Vec<[u8; 3]> { hex.iter().map(|h| hex_rgb(h)).collect() };
    let grass = bands(&GRASS_BANDS);
    let mud = bands(&MUD_BANDS);
    let water = bands(&SEA_BANDS);
    // The cell polygon at the radius the UVs actually reach.
    let unit = TILE_PX as f64 / 2.0 * UV_INSET;

    let style = Style {
        grass: [grass[0], grass[1], grass[2]],
        mud: [mud[0], mud[1], mud[2]],
        water: [water[0], water[1], water[2], water[3], water[4]],
        gp: probe(sphere, seed, GRASS_OCT, GRASS_F0),
        sp: probe(sphere, seed, SEA_OCT, SEA_F0),
        mp: probe(sphere, seed + MUD_SALT, MUD_OCT, MUD_F0),
        seed,
        // Hash lattice slightly finer than a texel, keyed on world position.
        k: DITHER_LATTICE / frames.px,
        pentagon: poly_edges(5, unit),
        hexagon: poly_edges(6, unit),
    };

    let mut atlas = Atlas {
        texture: Texture {
            image: RgbaImage::new(cols * TILE_PX as u32, rows * TILE_PX as u32),
            wrap_s: Wrap::Clamp,
            wrap_t: Wrap::Clamp,
            filter: Filter::Nearest,
            repeat: [1.0, 1.0],
        },
        cols,
        rows,
        cells,
        fields: compute_fields(sphere, snapshot),
        style,
    };
    for i in 0..count as u32 {
        atlas.paint(sphere, frames, snapshot, TileId(i));
    }
    atlas
}

impl Atlas {
    /// Repaints after a level change. The coastline moved, so the shading of everything within the
    /// fade radius changes with it, not just the tile that was clicked. Returns how many cells were
    /// repainted.
    ///
    /// `frames` must already reflect the new levels, as the prototype recomputes them first.
    pub fn refresh(
        &mut self,
        sphere: &HexSphere,
        frames: &Frames,
        snapshot: &WorldSnapshot,
        tile: TileId,
    ) -> usize {
        self.fields = compute_fields(sphere, snapshot);
        let mut seen = vec![tile];
        let mut ring = vec![tile];
        for _ in 0..=SEA_FADE {
            // The sea shelf reaches furthest.
            let mut next = Vec::new();
            for &id in &ring {
                for &j in &sphere.tiles[id.index()].neighbors {
                    if !seen.contains(&j) {
                        seen.push(j);
                        next.push(j);
                    }
                }
            }
            ring = next;
        }
        for &id in &seen {
            self.paint(sphere, frames, snapshot, id);
        }
        seen.len()
    }

    /// Repaints after a cover change. No coastline moved, so this skips the coast rebuild. It does
    /// take in one ring of neighbours, because the bare earth is allowed to spill across a border.
    pub fn repaint(
        &mut self,
        sphere: &HexSphere,
        frames: &Frames,
        snapshot: &WorldSnapshot,
        tiles: &[TileId],
    ) {
        compute_built(sphere, snapshot, &mut self.fields);
        let mut seen: Vec<TileId> = Vec::new();
        for &t in tiles {
            if !seen.contains(&t) {
                seen.push(t);
            }
            for &j in &sphere.tiles[t.index()].neighbors {
                if !seen.contains(&j) {
                    seen.push(j);
                }
            }
        }
        for &id in &seen {
            self.paint(sphere, frames, snapshot, id);
        }
    }

    /// The prototype's `paint`: one tile's cell, texel by texel.
    fn paint(&mut self, sphere: &HexSphere, frames: &Frames, snapshot: &WorldSnapshot, id: TileId) {
        let tile = sphere.tile(id);
        let frame = &frames.tiles[id.index()];
        let plane = facet_plane(tile, frame);
        let sea = snapshot.tiles[id.index()].terrain.level() < 0;
        let houses = is_built(snapshot, id.index()) > 0.0;
        let style = &self.style;
        let fields = &self.fields;
        let base: &[[u8; 3]] = if sea {
            &style.water[..]
        } else {
            &style.grass[..]
        };
        let rng = if sea { style.sp } else { style.gp };
        let edges: &[Edge] = if tile.sides() == 5 {
            &style.pentagon
        } else {
            &style.hexagon
        };
        let r_circum = frame.unit_circum;
        let n = tile.sides();
        let [col, row] = self.cells[id.index()];
        let ox = col * TILE_PX as u32;
        let oy = row * TILE_PX as u32;
        let half = TILE_PX as f64 / 2.0;

        // Corner positions in the same local frame the texel inverse produces.
        let corner: Vec<[f64; 2]> = tile
            .corners
            .iter()
            .map(|&c| [dot(c, tile.e1) / r_circum, dot(c, tile.e2) / r_circum])
            .collect();
        // Already sorted, increasing.
        let ang: Vec<f64> = corner.iter().map(|q| libm::atan2(q[1], q[0])).collect();

        for py in 0..TILE_PX {
            for px in 0..TILE_PX {
                let x = px as f64 + 0.5 - half;
                let y = py as f64 + 0.5 - half;
                let mut inset = f64::INFINITY;
                for e in edges {
                    inset = inset.min(e.d - (e.nx * x + e.ny * y));
                }
                if inset < -1.0 {
                    // Outside the sampled window: one flat tone, and the noise is skipped entirely.
                    let q = if !sea && houses { &style.mud[..] } else { base };
                    let c = q[0];
                    self.texture
                        .image
                        .put(ox + px as u32, oy + py as u32, [c[0], c[1], c[2], 255]);
                    continue;
                }
                let dir = texel_dir(tile, frame, px, py, &plane);
                let mut pal: &[[u8; 3]] = base;
                let r = hash3(
                    (dir[0] * style.k).floor(),
                    (dir[1] * style.k).floor(),
                    (dir[2] * style.k).floor(),
                );
                let (oct, f0) = if sea {
                    (SEA_OCT, SEA_F0)
                } else {
                    (GRASS_OCT, GRASS_F0)
                };
                let mut d = (fbm3(dir, style.seed, oct, f0) - rng.lo) / rng.span;
                d += (r - 0.5) * if sea { SEA_DITHER } else { GRASS_DITHER };

                // Coast field, interpolated over the fan triangle this texel lies in. Along a
                // shared edge the centre weight is zero, so the value depends only on the two
                // shared corners and matches from both sides.
                let lx = ((px as f64 + 0.5) / half - 1.0) / UV_INSET;
                let ly = (1.0 - (py as f64 + 0.5) / half) / UV_INSET;
                let th = libm::atan2(ly, lx);
                let mut k = n - 1;
                for i in 0..n - 1 {
                    if th >= ang[i] && th < ang[i + 1] {
                        k = i;
                        break;
                    }
                }
                let p = corner[k];
                let q = corner[(k + 1) % n];
                let det = p[0] * q[1] - p[1] * q[0];
                let wa = ((lx * q[1] - ly * q[0]) / det).max(0.0);
                let wb = ((p[0] * ly - p[1] * lx) / det).max(0.0);
                let wc = (1.0 - wa - wb).max(0.0);
                // JavaScript's `|| 1`: a degenerate fan falls back to a total weight of one.
                let total = wa + wb + wc;
                let s = if total > 0.0 { total } else { 1.0 };
                let (cp, cm) = if sea {
                    (
                        &fields.corner_shallow[id.index()],
                        fields.shallow[id.index()],
                    )
                } else {
                    (&fields.corner_prox[id.index()], fields.prox[id.index()])
                };
                let prox = (wa * cp[k] + wb * cp[(k + 1) % n] + wc * cm) / s;
                if sea {
                    d += prox * SEA_SHALLOW; // shoal toward the land
                } else {
                    d -= pow(prox, COAST_TIGHT) * COAST_DARKEN;
                }

                // Bare earth wherever the built-up field beats a noisy threshold. Only tiles at or
                // beside a village have any, so the extra noise lookup is skipped for the whole
                // rest of the planet.
                let cb = &fields.corner_built[id.index()];
                let built = (wa * cb[k] + wb * cb[(k + 1) % n] + wc * fields.built[id.index()]) / s;
                if !sea && built > 0.0 {
                    let mn = (fbm3(dir, style.seed + MUD_SALT, MUD_OCT, MUD_F0) - style.mp.lo)
                        / style.mp.span;
                    if built > MUD_EDGE + (mn - 0.5) * MUD_SCATTER + (r - 0.5) * MUD_DITHER {
                        pal = &style.mud[..];
                    }
                }

                let mut b = (d * pal.len() as f64).floor();
                if !sea && r > 1.0 - SPECKLE {
                    b += 1.0; // lone flecks catching the light
                }
                let c = pal[b.max(0.0).min(pal.len() as f64 - 1.0) as usize];
                self.texture
                    .image
                    .put(ox + px as u32, oy + py as u32, [c[0], c[1], c[2], 255]);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_hexsphere::compute_tile_frames;
    use cl_model::{Cover, Terrain, TileState};

    use super::*;

    /// A hemisphere of land with two villages on it: a real coastline, a sea shelf and a bare-earth
    /// fringe, without depending on `cl-worldgen`.
    fn world() -> (HexSphere, WorldSnapshot) {
        let sphere = HexSphere::build(3);
        let tiles: Vec<TileState> = sphere
            .tiles
            .iter()
            .map(|t| TileState {
                terrain: if t.center[2] > 0.0 {
                    Terrain::Land
                } else {
                    Terrain::Sea
                },
                ..TileState::default()
            })
            .collect();
        let mut snapshot = WorldSnapshot {
            frequency: 3,
            seed: 4242,
            tiles,
        };
        for id in snapshot.ids().collect::<Vec<_>>() {
            if snapshot.tile(id).terrain.is_land() && id.index() % 17 == 0 {
                snapshot.tile_mut(id).cover = Cover::Town;
            }
        }
        (sphere, snapshot)
    }

    fn build(sphere: &HexSphere, snapshot: &WorldSnapshot) -> Atlas {
        let frames = compute_tile_frames(sphere, &snapshot.levels());
        build_terrain_atlas(sphere, &frames, snapshot, f64::from(snapshot.seed))
    }

    /// The prototype's own claim about `refresh`: after a level change it leaves the atlas in the
    /// state a full rebuild would, and it touches the tile plus `SEA_FADE + 1` rings to get there.
    #[test]
    fn refresh_equals_a_full_rebuild_after_a_level_change() {
        let (sphere, snapshot) = world();
        let mut atlas = build(&sphere, &snapshot);
        let flipped = sphere
            .tiles
            .iter()
            .find(|t| {
                snapshot.tile(t.id).terrain.is_land()
                    && t.neighbors
                        .iter()
                        .any(|&j| !snapshot.tile(j).terrain.is_land())
            })
            .expect("the hemisphere has a coast")
            .id;

        let mut changed = snapshot.clone();
        changed.tile_mut(flipped).terrain = Terrain::Sea;
        changed.tile_mut(flipped).cover = Cover::None;
        let frames = compute_tile_frames(&sphere, &changed.levels());
        let touched = atlas.refresh(&sphere, &frames, &changed, flipped);

        let mut ball = vec![flipped];
        let mut ring = vec![flipped];
        for _ in 0..=SEA_FADE {
            let mut next = Vec::new();
            for &id in &ring {
                for &j in &sphere.tiles[id.index()].neighbors {
                    if !ball.contains(&j) {
                        ball.push(j);
                        next.push(j);
                    }
                }
            }
            ring = next;
        }
        assert_eq!(
            touched,
            ball.len(),
            "refresh repaints the tile and its rings"
        );
        assert_eq!(atlas, build(&sphere, &changed), "refresh matches a rebuild");
    }

    /// `repaint` skips the coast rebuild, so this pins that the one ring it does take in is enough
    /// for the bare earth that spills across a hex border.
    #[test]
    fn repaint_equals_a_full_rebuild_after_a_cover_change() {
        let (sphere, snapshot) = world();
        let mut atlas = build(&sphere, &snapshot);
        let built = sphere
            .tiles
            .iter()
            .find(|t| {
                snapshot.tile(t.id).cover == Cover::None && snapshot.tile(t.id).terrain.is_land()
            })
            .expect("the hemisphere has empty land")
            .id;

        let mut changed = snapshot.clone();
        changed.tile_mut(built).cover = Cover::Town;
        let frames = compute_tile_frames(&sphere, &changed.levels());
        atlas.repaint(&sphere, &frames, &changed, &[built]);
        assert_eq!(atlas, build(&sphere, &changed), "repaint matches a rebuild");

        changed.tile_mut(built).cover = Cover::None;
        atlas.repaint(&sphere, &frames, &changed, &[built]);
        assert_eq!(atlas, build(&sphere, &snapshot), "and gives the grass back");
    }

    /// A pentagon and a hexagon cell both fill every texel, and the cells past the last tile stay
    /// transparent as the prototype's cleared canvas leaves them.
    #[test]
    fn every_cell_is_opaque_and_the_spare_ones_are_not() {
        let (sphere, snapshot) = world();
        let atlas = build(&sphere, &snapshot);
        assert_eq!((atlas.cols, atlas.rows), (10, 10));
        for id in snapshot.ids() {
            let [col, row] = atlas.cells[id.index()];
            for py in 0..TILE_PX as u32 {
                for px in 0..TILE_PX as u32 {
                    let texel = atlas
                        .texture
                        .image
                        .get(col * TILE_PX as u32 + px, row * TILE_PX as u32 + py);
                    assert_eq!(texel[3], 255, "tile {id} texel ({px},{py})");
                }
            }
        }
        let spare = atlas.cells.len() as u32;
        assert!(
            spare < atlas.cols * atlas.rows,
            "frequency 3 leaves spare cells"
        );
        let [col, row] = [spare % atlas.cols, spare / atlas.cols];
        assert_eq!(
            atlas
                .texture
                .image
                .get(col * TILE_PX as u32, row * TILE_PX as u32),
            [0, 0, 0, 0]
        );
    }
}
