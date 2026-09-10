#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Compares ported builders with `fixtures/scenery/*-sky.json`, extracted from the prototype.
//! The hash covers the little-endian bytes of the `f32` attribute arrays, so it is exact.

use std::fs;
use std::path::PathBuf;

use cl_hexsphere::{HexSphere, compute_tile_frames};
use cl_model::{Cover, Terrain, TileId, TileState, WorldSnapshot};
use cl_pixelart::build_terrain_atlas;
use cl_scenery::{build_atmosphere, build_cloud_shell, build_fields, build_terrain};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixture(rel: &str) -> Value {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "fixtures",
        "scenery",
        rel,
    ]
    .iter()
    .collect();
    serde_json::from_str(
        &fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
    )
    .unwrap()
}

fn levels(tag: &str) -> Vec<i32> {
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
    let v: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    v["level"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap() as i32)
        .collect()
}

fn sha256_f32(values: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for v in values {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    hex::encode(Sha256::digest(&bytes))
}

/// Asserts one attribute against the fixture summary: length, exact hash, and the first floats.
fn check_attribute(name: &str, got: &[f32], want: &Value) {
    assert_eq!(
        got.len(),
        want["len"].as_u64().unwrap() as usize,
        "{name} length"
    );
    let head: Vec<f32> = want["head"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap() as f32)
        .collect();
    assert_eq!(
        &got[..head.len()],
        &head[..],
        "{name} first {} floats",
        head.len()
    );
    assert_eq!(
        sha256_f32(got),
        want["sha256_f32"].as_str().unwrap(),
        "{name} sha256 of f32 bytes"
    );
}

#[test]
fn atmosphere_and_cloud_shell_match_prototype() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let f = fixture(&format!("{tag}-sky.json"));
        let n = f["n"].as_u64().unwrap() as u8;
        let sphere = HexSphere::build(n);
        let frames = compute_tile_frames(&sphere, &levels(tag));
        assert_eq!(
            frames.px.to_bits(),
            f["px"].as_f64().unwrap().to_bits(),
            "{tag} px"
        );

        let air = build_atmosphere(&sphere, frames.px);
        assert_eq!(
            air.radius,
            f["atmosphere"]["radius"].as_f64().unwrap(),
            "{tag} atmosphere radius"
        );
        check_attribute(
            "atmosphere.position",
            &air.mesh.positions,
            &f["atmosphere"]["geometry"]["position"],
        );
        check_attribute(
            "atmosphere.normal",
            &air.mesh.normals,
            &f["atmosphere"]["geometry"]["normal"],
        );
        assert!(air.mesh.validate().is_ok());

        let shell = build_cloud_shell(&sphere, &frames, frames.px);
        assert_eq!(
            shell.radius,
            f["cloudShell"]["radius"].as_f64().unwrap(),
            "{tag} cloud shell radius"
        );
        check_attribute(
            "cloudShell.position",
            &shell.mesh.positions,
            &f["cloudShell"]["geometry"]["position"],
        );
        check_attribute(
            "cloudShell.uv",
            &shell.mesh.uvs,
            &f["cloudShell"]["geometry"]["uv"],
        );
        check_attribute(
            "cloudShell.normal",
            &shell.mesh.normals,
            &f["cloudShell"]["geometry"]["normal"],
        );
        assert!(shell.mesh.validate().is_ok());
    }
}

/// The prototype world behind a fixture tag: levels and cover straight from `fixtures/worldgen`, so
/// the mesh is pinned against the prototype's own world and not against a second port.
fn world(tag: &str) -> (HexSphere, WorldSnapshot) {
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
    let v: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let frequency = v["n"].as_u64().unwrap() as u8;
    let seed = v["seed"].as_u64().unwrap() as u32;
    let tiles = v["level"]
        .as_array()
        .unwrap()
        .iter()
        .zip(v["cover"].as_array().unwrap())
        .map(|(level, cover)| TileState {
            terrain: Terrain::from_level(level.as_i64().unwrap() as i32),
            cover: match cover.as_str() {
                None => Cover::None,
                Some("houses") => Cover::Town,
                Some("forest") => Cover::Forest,
                Some("fields") => Cover::Field,
                Some(other) => panic!("unknown cover {other}"),
            },
            ..TileState::default()
        })
        .collect();
    (
        HexSphere::build(frequency),
        WorldSnapshot {
            frequency,
            seed,
            tiles,
        },
    )
}

fn sha256_ids(ids: &[TileId]) -> String {
    let joined: Vec<String> = ids.iter().map(|i| i.0.to_string()).collect();
    hex::encode(Sha256::digest(joined.join(",").as_bytes()))
}

#[test]
fn terrain_meshes_match_prototype_worlds() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let f = fixture(&format!("{tag}-terrain.json"));
        let (sphere, snapshot) = world(tag);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        assert_eq!(
            frames.px.to_bits(),
            f["px"].as_f64().unwrap().to_bits(),
            "{tag} px"
        );
        let atlas = build_terrain_atlas(&sphere, &frames, &snapshot, f64::from(snapshot.seed));
        let terrain = build_terrain(&sphere, &frames, &snapshot, &atlas);

        for (name, mesh) in [
            ("mesh", &terrain.mesh),
            ("walls", &terrain.walls),
            ("foam", &terrain.foam),
            ("edges", &terrain.edges),
        ] {
            let want = &f[name];
            check_attribute(
                &format!("{tag} {name}.position"),
                &mesh.positions,
                &want["position"],
            );
            if !want["uv"].is_null() {
                check_attribute(&format!("{tag} {name}.uv"), &mesh.uvs, &want["uv"]);
            }
            if !want["color"].is_null() {
                check_attribute(&format!("{tag} {name}.color"), &mesh.colors, &want["color"]);
            }
            if !want["normal"].is_null() {
                check_attribute(
                    &format!("{tag} {name}.normal"),
                    &mesh.normals,
                    &want["normal"],
                );
            }
            assert!(
                mesh.validate().is_ok(),
                "{tag} {name}: {:?}",
                mesh.validate()
            );
        }

        assert_eq!(
            terrain.face_tile().len(),
            f["faceTileLen"].as_u64().unwrap() as usize,
            "{tag} faceTile length"
        );
        assert_eq!(
            sha256_ids(terrain.face_tile()),
            f["faceTileHash"].as_str().unwrap(),
            "{tag} faceTile"
        );
        assert_eq!(
            terrain.wall_tile().len(),
            f["wallTileLen"].as_u64().unwrap() as usize,
            "{tag} wallTile length"
        );
        assert_eq!(
            sha256_ids(terrain.wall_tile()),
            f["wallTileHash"].as_str().unwrap(),
            "{tag} wallTile"
        );

        let starts: Vec<usize> = f["vertexStart"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect();
        let counts: Vec<usize> = f["vertexCount"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(terrain.vertex_start, starts, "{tag} vertexStart");
        assert_eq!(terrain.vertex_count, counts, "{tag} vertexCount");
    }
}

#[test]
fn farmland_matches_prototype_worlds() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let f = fixture(&format!("{tag}-cover.json"));
        let want = &f["fields"];
        let (sphere, snapshot) = world(tag);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let fields = build_fields(&sphere, &frames, &snapshot, frames.px);

        if want.is_null() {
            assert!(fields.is_none(), "{tag}: the prototype grew no farmland");
            continue;
        }
        let fields = fields.unwrap_or_else(|| panic!("{tag}: expected farmland"));

        for (name, got, key) in [
            ("zones", fields.zones, "zones"),
            ("parcels", fields.parcels, "parcels"),
            ("fences", fields.fences, "fences"),
            ("tiles", fields.tiles, "tiles"),
        ] {
            assert_eq!(
                got,
                want[key].as_u64().unwrap() as usize,
                "{tag} field {name}"
            );
        }

        check_attribute(
            &format!("{tag} fields.position"),
            &fields.surface.positions,
            &want["surface"]["position"],
        );
        check_attribute(
            &format!("{tag} fields.uv"),
            &fields.surface.uvs,
            &want["surface"]["uv"],
        );
        check_attribute(
            &format!("{tag} fields.color"),
            &fields.surface.colors,
            &want["surface"]["color"],
        );
        check_attribute(
            &format!("{tag} fields.normal"),
            &fields.surface.normals,
            &want["surface"]["normal"],
        );
        assert!(fields.surface.validate().is_ok());

        if want["posts"].is_null() {
            assert!(fields.posts.is_empty(), "{tag}: no fence in the prototype");
        } else {
            check_attribute(
                &format!("{tag} posts.position"),
                &fields.posts.positions,
                &want["posts"]["position"],
            );
            check_attribute(
                &format!("{tag} posts.color"),
                &fields.posts.colors,
                &want["posts"]["color"],
            );
            check_attribute(
                &format!("{tag} posts.normal"),
                &fields.posts.normals,
                &want["posts"]["normal"],
            );
            assert!(fields.posts.validate().is_ok());
        }
    }
}
