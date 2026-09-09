//! Seeded continents and initial cover for a hex-sphere world, prototype sections 3 and 4b.
#![forbid(unsafe_code)]

use cl_hexsphere::HexSphere;
use cl_model::{Cover, Terrain, TileState, WorldSnapshot};
use cl_noise::Mulberry32;
use cl_noise::js::{round, sin, to_int32};
use cl_noise::vec::{dot, norm};

/// Share of tiles that come out as land.
pub const LAND_FRACTION: f64 = 0.42;
/// Share of land that starts out as villages.
pub const TOWN_SHARE: f64 = 0.08;
/// Share of land that starts out as forest.
pub const WOOD_SHARE: f64 = 0.20;
/// Share of land that starts out as fields.
pub const FARM_SHARE: f64 = 0.16;

/// Continents from seven directional waves over the sphere, then one smoothing pass so the
/// coastline is not confetti. The threshold is a quantile, which pins the land ratio whatever the
/// waves do. Returns one elevation level per tile: `0` land, `-1` sea.
pub fn generate_terrain(sphere: &HexSphere, seed: f64) -> Vec<i32> {
    generate_terrain_with_fraction(sphere, seed, LAND_FRACTION)
}

/// [`generate_terrain`] with an explicit land share (`0.0..=1.0`); levels set it in percent.
pub fn generate_terrain_with_fraction(
    sphere: &HexSphere,
    seed: f64,
    land_fraction: f64,
) -> Vec<i32> {
    let mut rnd = Mulberry32::new(seed);
    let mut waves = Vec::with_capacity(7);
    for _ in 0..7 {
        let x = rnd.next_f64() * 2.0 - 1.0;
        let y = rnd.next_f64() * 2.0 - 1.0;
        let z = rnd.next_f64() * 2.0 - 1.0;
        let d = norm([x, y, z]);
        let f = 1.2 + rnd.next_f64() * 4.2;
        let p = rnd.next_f64() * std::f64::consts::PI * 2.0;
        waves.push((d, f, p));
    }
    let e: Vec<f64> = sphere
        .tiles
        .iter()
        .map(|t| {
            let mut s = 0.0;
            for &(d, f, p) in &waves {
                s += sin(f * dot(t.center, d) * std::f64::consts::PI + p) / f;
            }
            s
        })
        .collect();
    let mut sorted = e.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("wave sums are finite"));
    let cut_index =
        ((sphere.len() as f64 * (1.0 - land_fraction)).floor() as usize).min(sphere.len() - 1);
    let cut = sorted[cut_index];
    let snapshot: Vec<i32> = e.iter().map(|&v| if v > cut { 0 } else { -1 }).collect();

    let mut levels = snapshot.clone();
    for (i, t) in sphere.tiles.iter().enumerate() {
        let land = t
            .neighbors
            .iter()
            .filter(|j| snapshot[j.index()] == 0)
            .count();
        if snapshot[i] == 0 && land <= 1 {
            levels[i] = -1; // lone islet
        }
        if snapshot[i] == -1 && land >= t.sides() - 1 {
            levels[i] = 0; // lone puddle
        }
    }
    levels
}

/// Farmland, woods and villages arrive in clumps, because one lone hex of wheat is not a farm:
/// the cover renderers merge neighbours into single zones, and that only shows when tiles come in
/// groups. Villages first, then woods, then farmland in what is left.
pub fn seed_cover(sphere: &HexSphere, levels: &[i32], seed: f64) -> Vec<Cover> {
    assert_eq!(levels.len(), sphere.len(), "one level per tile");
    let mut rnd = Mulberry32::from_i32(to_int32(seed) ^ 0x5bf0_3635);
    let mut cover = vec![Cover::None; sphere.len()];
    let land: Vec<usize> = (0..sphere.len()).filter(|&i| levels[i] >= 0).collect();
    if land.is_empty() {
        return cover;
    }
    for (kind, share, cling) in [
        (Cover::Town, TOWN_SHARE, 0.92),
        (Cover::Forest, WOOD_SHARE, 0.90),
        (Cover::Field, FARM_SHARE, 0.85),
    ] {
        let mut budget = round(land.len() as f64 * share) as i64;
        let mut guard = 0;
        while budget > 0 && guard < 900 {
            guard += 1;
            let s = land[to_int32(rnd.next_f64() * land.len() as f64) as usize];
            if cover[s] != Cover::None {
                continue;
            }
            cover[s] = kind;
            budget -= 1;
            let mut ring = vec![s];
            let mut p = cling;
            while budget > 0 && !ring.is_empty() && p > 0.2 {
                let mut next = Vec::new();
                for &t in &ring {
                    for j in &sphere.tiles[t].neighbors {
                        if budget <= 0 {
                            break;
                        }
                        let n = j.index();
                        if cover[n] != Cover::None || levels[n] < 0 {
                            continue;
                        }
                        if rnd.next_f64() > p {
                            continue;
                        }
                        cover[n] = kind;
                        budget -= 1;
                        next.push(n);
                    }
                }
                ring = next;
                p *= 0.62;
            }
        }
    }
    cover
}

/// Terrain and cover for `(n, seed)` as a snapshot with no owners or units: the prototype's
/// `makeWorld` before any mesh is built.
pub fn generate_world(n: u8, seed: u32) -> WorldSnapshot {
    let sphere = HexSphere::build(n);
    let levels = generate_terrain(&sphere, f64::from(seed));
    let cover = seed_cover(&sphere, &levels, f64::from(seed));
    let tiles = levels
        .iter()
        .zip(cover)
        .map(|(&level, cover)| TileState {
            terrain: Terrain::from_level(level),
            cover,
            ..TileState::default()
        })
        .collect();
    WorldSnapshot {
        frequency: n,
        seed,
        tiles,
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn world_is_valid_and_land_ratio_is_near_target() {
        let w = generate_world(4, 31676);
        assert!(w.validate().is_ok());
        let land = w.tiles.iter().filter(|t| t.terrain.is_land()).count();
        let ratio = land as f64 / w.tiles.len() as f64;
        assert!((0.3..0.55).contains(&ratio), "land ratio {ratio}");
        assert!(
            w.tiles
                .iter()
                .all(|t| t.terrain.is_land() || t.cover == Cover::None)
        );
    }
}
