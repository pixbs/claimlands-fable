#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Replays `fixtures/hexsphere/*.json` and the frame fixtures of `fixtures/scenery`, extracted from
//! the prototype by `reference/harness/extract.mjs`.

use std::fs;
use std::path::PathBuf;

use cl_hexsphere::{HexSphere, compute_tile_frames, facet_plane, texel_dir, tiles_around};
use cl_model::TileId;
use cl_noise::js::floor_half_up;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixture(rel: &str) -> Value {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "fixtures", rel]
        .iter()
        .collect();
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid fixture json")
}

fn f64s(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn ids(v: &Value) -> Vec<TileId> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| TileId(x.as_u64().unwrap() as u32))
        .collect()
}

/// The harness' `hashQ6`: values quantised to 1e-6 with `floor(x + 0.5)`, joined by commas.
fn hash_q6(values: &[f64]) -> String {
    let mut s = String::new();
    for &x in values {
        let q = floor_half_up(x * 1e6);
        s.push_str(&format!("{},", q as i64));
    }
    hex::encode(Sha256::digest(s.as_bytes()))
}

#[test]
fn every_frequency_matches_prototype_topology() {
    for n in 2..=12u8 {
        let f = fixture(&format!("hexsphere/n{n}.json"));
        let s = HexSphere::build(n);
        assert_eq!(
            s.len(),
            f["count"].as_u64().unwrap() as usize,
            "n={n} count"
        );
        assert_eq!(s.pentagons(), ids(&f["pentagons"]), "n={n} pentagons");
        assert_eq!(
            tiles_around(&s),
            f["around"].as_u64().unwrap() as u32,
            "n={n} around"
        );
        for (t, sides) in s.tiles.iter().zip(f["sides"].as_array().unwrap()) {
            assert_eq!(
                t.sides(),
                sides.as_u64().unwrap() as usize,
                "n={n} tile {} sides",
                t.id
            );
        }
        for (t, nb) in s.tiles.iter().zip(f["neighbors"].as_array().unwrap()) {
            assert_eq!(t.neighbors, ids(nb), "n={n} tile {} neighbors", t.id);
        }
        for (t, en) in s.tiles.iter().zip(f["edgeNeighbors"].as_array().unwrap()) {
            assert_eq!(
                t.edge_neighbors,
                ids(en),
                "n={n} tile {} edgeNeighbors",
                t.id
            );
        }
        for (t, ct) in s.tiles.iter().zip(f["cornerTiles"].as_array().unwrap()) {
            let expected: Vec<[TileId; 3]> = ct
                .as_array()
                .unwrap()
                .iter()
                .map(|c| <[TileId; 3]>::try_from(ids(c)).unwrap())
                .collect();
            assert_eq!(t.corner_tiles, expected, "n={n} tile {} cornerTiles", t.id);
        }
        let mut cee: Vec<f64> = Vec::new();
        let mut corners: Vec<f64> = Vec::new();
        for t in &s.tiles {
            cee.extend(t.center);
            cee.extend(t.e1);
            cee.extend(t.e2);
            for c in &t.corners {
                corners.extend(c);
            }
        }
        assert_eq!(
            hash_q6(&cee),
            f["sha256_q6_centers_e1_e2"].as_str().unwrap(),
            "n={n} centers/e1/e2 hash"
        );
        assert_eq!(
            hash_q6(&corners),
            f["sha256_q6_corners"].as_str().unwrap(),
            "n={n} corners hash"
        );
    }
}

#[test]
fn small_frequencies_match_bit_for_bit() {
    let mut mismatches = Vec::new();
    for n in 2..=6u8 {
        let f = fixture(&format!("hexsphere/n{n}.json"));
        let s = HexSphere::build(n);
        let g = cl_hexsphere::geodesic(n);
        for (i, v) in f["geodesicVerts"].as_array().unwrap().iter().enumerate() {
            if g.verts[i].to_vec() != f64s(v) {
                mismatches.push(format!("n={n} geodesic vertex {i}"));
            }
        }
        for (i, tri) in f["geodesicTris"].as_array().unwrap().iter().enumerate() {
            let expected: Vec<u32> = tri
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect();
            if g.tris[i].to_vec() != expected {
                mismatches.push(format!("n={n} geodesic triangle {i}"));
            }
        }
        for (t, ft) in s.tiles.iter().zip(f["tiles"].as_array().unwrap()) {
            let check = |name: &str, got: &[f64], want: &Value| {
                if got != f64s(want).as_slice() {
                    Some(format!(
                        "n={n} tile {} {name}: got {got:?} want {want}",
                        t.id
                    ))
                } else {
                    None
                }
            };
            mismatches.extend(check("center", &t.center, &ft["center"]));
            mismatches.extend(check("e1", &t.e1, &ft["e1"]));
            mismatches.extend(check("e2", &t.e2, &ft["e2"]));
            for (k, c) in ft["corners"].as_array().unwrap().iter().enumerate() {
                mismatches.extend(check(&format!("corner {k}"), &t.corners[k], c));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches, first: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(10)].join("\n")
    );
}

#[test]
fn frames_match_fixture_worlds() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let world = fixture(&format!("worldgen/{tag}.json"));
        let expect = fixture(&format!("scenery/{tag}-frames.json"));
        let n = world["n"].as_u64().unwrap() as u8;
        let levels: Vec<i32> = world["level"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap() as i32)
            .collect();
        let s = HexSphere::build(n);
        let frames = compute_tile_frames(&s, &levels);
        assert_eq!(
            frames.px.to_bits(),
            expect["px"].as_f64().unwrap().to_bits(),
            "{tag} px"
        );
        assert_eq!(
            tiles_around(&s),
            expect["around"].as_u64().unwrap() as u32,
            "{tag} around"
        );

        let mut packed: Vec<f64> = Vec::new();
        for (t, fr) in s.tiles.iter().zip(&frames.tiles) {
            let pl = facet_plane(t, fr);
            packed.push(fr.radius);
            packed.extend(fr.mid);
            packed.extend(fr.normal);
            packed.extend([fr.apothem, fr.texel, fr.unit_circum, pl.d, pl.dc]);
        }
        assert_eq!(
            hash_q6(&packed),
            expect["framesHash"].as_str().unwrap(),
            "{tag} frames hash"
        );

        if let Some(list) = expect["frames"].as_array() {
            for ((t, fr), want) in s.tiles.iter().zip(&frames.tiles).zip(list) {
                let pl = facet_plane(t, fr);
                assert_eq!(
                    fr.radius,
                    want["radius"].as_f64().unwrap(),
                    "{tag} tile {} radius",
                    t.id
                );
                assert_eq!(
                    fr.mid.to_vec(),
                    f64s(&want["mid"]),
                    "{tag} tile {} mid",
                    t.id
                );
                assert_eq!(
                    fr.normal.to_vec(),
                    f64s(&want["normal"]),
                    "{tag} tile {} normal",
                    t.id
                );
                assert_eq!(
                    fr.apothem,
                    want["apothem"].as_f64().unwrap(),
                    "{tag} tile {} apothem",
                    t.id
                );
                assert_eq!(
                    fr.texel,
                    want["texel"].as_f64().unwrap(),
                    "{tag} tile {} texel",
                    t.id
                );
                assert_eq!(
                    fr.unit_circum,
                    want["unitCircum"].as_f64().unwrap(),
                    "{tag} tile {} unitCircum",
                    t.id
                );
                assert_eq!(
                    pl.d,
                    want["plane"]["d"].as_f64().unwrap(),
                    "{tag} tile {} plane.d",
                    t.id
                );
                assert_eq!(
                    pl.dc,
                    want["plane"]["dc"].as_f64().unwrap(),
                    "{tag} tile {} plane.dc",
                    t.id
                );
            }
        }

        let dirs = expect["texelDirTiles0to2Row0"].as_array().unwrap();
        for (i, want) in dirs.iter().enumerate() {
            let tile = &s.tiles[i / 24];
            let fr = &frames.tiles[i / 24];
            let pl = facet_plane(tile, fr);
            let got = texel_dir(tile, fr, i % 24, 0, &pl);
            assert_eq!(
                got.to_vec(),
                f64s(want),
                "{tag} texelDir tile {} px {}",
                i / 24,
                i % 24
            );
        }
    }
}
