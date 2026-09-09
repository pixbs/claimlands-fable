#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Replays `fixtures/worldgen/*.json`, extracted from the prototype by `reference/harness/extract.mjs`.

use std::fs;
use std::path::PathBuf;

use cl_hexsphere::HexSphere;
use cl_model::Cover;
use cl_worldgen::{generate_terrain, generate_world, seed_cover};
use serde_json::Value;

fn fixture(tag: &str) -> Value {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "fixtures",
        "worldgen",
        &format!("{tag}.json"),
    ]
    .iter()
    .collect();
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid fixture json")
}

fn cover_of(v: &Value) -> Cover {
    match v.as_str() {
        None => Cover::None,
        Some("houses") => Cover::Town,
        Some("forest") => Cover::Forest,
        Some("fields") => Cover::Field,
        Some(other) => panic!("unknown cover {other}"),
    }
}

#[test]
fn terrain_and_cover_match_prototype_worlds() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let f = fixture(tag);
        let n = f["n"].as_u64().unwrap() as u8;
        let seed = f["seed"].as_f64().unwrap();
        let sphere = HexSphere::build(n);
        let levels = generate_terrain(&sphere, seed);
        let expected_levels: Vec<i32> = f["level"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap() as i32)
            .collect();
        let level_mismatches: Vec<usize> = (0..levels.len())
            .filter(|&i| levels[i] != expected_levels[i])
            .collect();
        assert!(
            level_mismatches.is_empty(),
            "{tag}: {} tiles differ in level, first {:?}",
            level_mismatches.len(),
            &level_mismatches[..level_mismatches.len().min(10)]
        );
        assert_eq!(
            levels.iter().filter(|&&l| l == 0).count(),
            f["land"].as_u64().unwrap() as usize,
            "{tag} land count"
        );

        let cover = seed_cover(&sphere, &levels, seed);
        let expected_cover: Vec<Cover> = f["cover"]
            .as_array()
            .unwrap()
            .iter()
            .map(cover_of)
            .collect();
        let cover_mismatches: Vec<usize> = (0..cover.len())
            .filter(|&i| cover[i] != expected_cover[i])
            .collect();
        assert!(
            cover_mismatches.is_empty(),
            "{tag}: {} tiles differ in cover, first {:?}",
            cover_mismatches.len(),
            &cover_mismatches[..cover_mismatches.len().min(10)]
        );

        let world = generate_world(n, seed as u32);
        assert_eq!(world.tiles.len(), levels.len());
        for (i, t) in world.tiles.iter().enumerate() {
            assert_eq!(t.terrain.level(), levels[i], "{tag} snapshot level {i}");
            assert_eq!(t.cover, cover[i], "{tag} snapshot cover {i}");
        }
    }
}
