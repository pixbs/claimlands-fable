#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Compares ported builders with `fixtures/scenery/*-sky.json`, extracted from the prototype.
//! The hash covers the little-endian bytes of the `f32` attribute arrays, so it is exact.

use std::fs;
use std::path::PathBuf;

use cl_hexsphere::{HexSphere, compute_tile_frames};
use cl_scenery::{build_atmosphere, build_cloud_shell};
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
