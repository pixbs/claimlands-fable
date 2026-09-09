//! Per-tile facet frames and the texel inverse, prototype `computeTileFrames`, `facetPlane`,
//! `texelDir` and `tilesAround`.

use cl_model::TileId;
use cl_model::world::{LEVEL_PX, RADIUS, TILE_PX, UV_INSET};
use cl_noise::js;
use cl_noise::vec::{V3, add, dot, len, mul, norm, sub};

use crate::{HexSphere, Tile};

/// Shell radius, facet centroid, flat normal, apothem and texel size of one tile at its level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileFrame {
    /// `RADIUS + level * step`: the shell this tile sits on.
    pub radius: f64,
    /// Polygon centroid scaled to the shell (the fan's centre vertex).
    pub mid: V3,
    /// One outward Newell normal for the whole facet.
    pub normal: V3,
    /// Mean distance from `mid` to the edge midpoints.
    pub apothem: f64,
    /// One texture pixel on this tile, in world units.
    pub texel: f64,
    /// Circumradius of the tile polygon on the unit sphere.
    pub unit_circum: f64,
}

/// Frames for every tile plus the two world-wide scales.
#[derive(Debug, Clone, PartialEq)]
pub struct Frames {
    /// One texture pixel in world units (mean over all tiles).
    pub px: f64,
    /// One elevation level in world units: `LEVEL_PX * px`.
    pub step: f64,
    /// Per-tile frames, indexed by tile id.
    pub tiles: Vec<TileFrame>,
}

/// The prototype's `computeTileFrames`. `levels` holds one elevation level per tile.
pub fn compute_tile_frames(sphere: &HexSphere, levels: &[i32]) -> Frames {
    assert_eq!(levels.len(), sphere.tiles.len(), "one level per tile");
    let unit = TILE_PX as f64 / 2.0 * UV_INSET;
    let mut circum = Vec::with_capacity(sphere.tiles.len());
    let mut mean_r = 0.0;
    for t in &sphere.tiles {
        let mut r: f64 = 0.0;
        for &p in &t.corners {
            r = r.max(len(sub(p, t.center)));
        }
        circum.push(r);
        mean_r += r;
    }
    mean_r /= sphere.tiles.len() as f64;
    let px = mean_r * RADIUS / unit;
    let step = LEVEL_PX * px;

    let tiles = sphere
        .tiles
        .iter()
        .zip(&circum)
        .zip(levels)
        .map(|((t, &unit_circum), &level)| {
            let sides = t.sides();
            let rad = RADIUS + f64::from(level) * step;
            let mut mid = [0.0, 0.0, 0.0];
            for &p in &t.corners {
                mid = add(mid, p);
            }
            let mid = mul(mid, rad / sides as f64);

            let mut nn = [0.0, 0.0, 0.0];
            for k in 0..sides {
                let p = t.corners[k];
                let q = t.corners[(k + 1) % sides];
                nn[0] += (p[1] - q[1]) * (p[2] + q[2]);
                nn[1] += (p[2] - q[2]) * (p[0] + q[0]);
                nn[2] += (p[0] - q[0]) * (p[1] + q[1]);
            }
            let mut normal = norm(nn);
            if dot(normal, t.center) < 0.0 {
                normal = mul(normal, -1.0);
            }

            let mut ap = 0.0;
            for k in 0..sides {
                let m = mul(add(t.corners[k], t.corners[(k + 1) % sides]), rad * 0.5);
                ap += len(sub(m, mid));
            }
            TileFrame {
                radius: rad,
                mid,
                normal,
                apothem: ap / sides as f64,
                texel: unit_circum * rad / unit,
                unit_circum,
            }
        })
        .collect();
    Frames { px, step, tiles }
}

/// The facet plane of a tile in unit-sphere scale: `dot(p, n) == d` on the facet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacetPlane {
    /// Facet normal.
    pub n: V3,
    /// Mean of `dot(corner, n)` over the corners.
    pub d: f64,
    /// `dot(center, n)`: how far the sphere point above the facet sits from the plane.
    pub dc: f64,
}

/// The prototype's `facetPlane`.
pub fn facet_plane(tile: &Tile, frame: &TileFrame) -> FacetPlane {
    let n = frame.normal;
    let mut d = 0.0;
    for &c in &tile.corners {
        d += dot(c, n);
    }
    FacetPlane {
        n,
        d: d / tile.sides() as f64,
        dc: dot(tile.center, n),
    }
}

/// Exact inverse of the atlas UV mapping: the direction covered by texel `(px, py)` of the tile's
/// cell, placed on the facet plane so that both tiles sharing an edge resolve an edge texel to the
/// same point.
pub fn texel_dir(tile: &Tile, frame: &TileFrame, px: usize, py: usize, plane: &FacetPlane) -> V3 {
    let half = TILE_PX as f64 / 2.0;
    let x = ((px as f64 + 0.5) / half - 1.0) / UV_INSET;
    let y = (1.0 - (py as f64 + 0.5) / half) / UV_INSET;
    let r = frame.unit_circum;
    let tang = add(mul(tile.e1, x * r), mul(tile.e2, y * r));
    let hgt = (plane.d - dot(tang, plane.n)) / plane.dc;
    norm(add(tang, mul(tile.center, hgt)))
}

/// Tiles a pawn would cross to circle the planet: the mean angular step between neighbouring tile
/// centres, inverted and rounded like `Math.round`.
pub fn tiles_around(sphere: &HexSphere) -> u32 {
    let mut sum = 0.0;
    let mut n = 0u32;
    for t in &sphere.tiles {
        for &j in &t.neighbors {
            if j < t.id {
                continue;
            }
            let d = dot(t.center, sphere.tile(j).center);
            sum += libm::acos(d.clamp(-1.0, 1.0));
            n += 1;
        }
    }
    js::round(2.0 * std::f64::consts::PI / (sum / f64::from(n))) as u32
}

/// Convenience: `tiles_around` needs ids to compare, so it lives here rather than on `Tile`.
impl HexSphere {
    /// Tile by id.
    pub fn tile(&self, id: TileId) -> &Tile {
        &self.tiles[id.index()]
    }
}
