#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Compares generated textures with `fixtures/pixelart/*.png` and constants with
//! `fixtures/constants.json`, both extracted from the prototype.

use std::fs;
use std::path::PathBuf;

use cl_model::RgbaImage;
use cl_pixelart::palette::{
    AIR_COLOR, CLIFF_ROWS, FIELD_CROPS, FOAM_ROWS, GRASS_BANDS, MUD_BANDS, SEA_BANDS,
};
use cl_pixelart::{Wrap, make_cliff_texture, make_field_texture, make_foam_texture};
use serde_json::Value;

/// Texels allowed to differ in strips that sample `hash2` (`sin` drift, see cl-noise tests).
const HASH2_TEXEL_BUDGET: usize = 2;

fn fixture_path(rel: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "..", "..", "fixtures", rel]
        .iter()
        .collect()
}

fn png(rel: &str) -> RgbaImage {
    let img = image::open(fixture_path(rel))
        .unwrap_or_else(|e| panic!("open {rel}: {e}"))
        .into_rgba8();
    RgbaImage {
        width: img.width(),
        height: img.height(),
        data: img.into_raw(),
    }
}

fn json(rel: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(fixture_path(rel)).unwrap()).unwrap()
}

fn differing_texels(a: &RgbaImage, b: &RgbaImage) -> Vec<(u32, u32)> {
    assert_eq!((a.width, a.height), (b.width, b.height), "size");
    let mut out = Vec::new();
    for y in 0..a.height {
        for x in 0..a.width {
            if a.get(x, y) != b.get(x, y) {
                out.push((x, y));
            }
        }
    }
    out
}

fn wrap_of(v: &Value) -> Wrap {
    match v.as_str().unwrap() {
        "RepeatWrapping" => Wrap::Repeat,
        "ClampToEdgeWrapping" => Wrap::Clamp,
        other => panic!("unknown wrap {other}"),
    }
}

#[test]
fn cliff_strip_matches_prototype() {
    let tex = make_cliff_texture();
    let diff = differing_texels(&tex.image, &png("pixelart/cliff.png"));
    assert!(
        diff.len() <= HASH2_TEXEL_BUDGET,
        "{} cliff texels differ: {diff:?}",
        diff.len()
    );
    let meta = json("pixelart/cliff.json");
    assert_eq!(tex.wrap_s, wrap_of(&meta["wrapS"]));
    assert_eq!(tex.wrap_t, wrap_of(&meta["wrapT"]));
    assert_eq!(meta["magFilter"], "NearestFilter");
    assert_eq!(meta["generateMipmaps"], false);
}

#[test]
fn foam_sheet_matches_prototype() {
    let tex = make_foam_texture();
    let diff = differing_texels(&tex.image, &png("pixelart/foam.png"));
    assert!(
        diff.len() <= HASH2_TEXEL_BUDGET,
        "{} foam texels differ: {diff:?}",
        diff.len()
    );
    let meta = json("pixelart/foam.json");
    assert_eq!(tex.wrap_s, wrap_of(&meta["wrapS"]));
    assert_eq!(tex.wrap_t, wrap_of(&meta["wrapT"]));
    assert_eq!(tex.repeat[1], meta["repeat"]["y"].as_f64().unwrap());
}

#[test]
fn field_strip_matches_prototype_exactly() {
    let tex = make_field_texture();
    let diff = differing_texels(&tex.image, &png("pixelart/field.png"));
    assert!(diff.is_empty(), "field texels differ: {diff:?}");
    let meta = json("pixelart/field.json");
    assert_eq!(tex.wrap_s, wrap_of(&meta["wrapS"]));
    assert_eq!(tex.wrap_t, wrap_of(&meta["wrapT"]));
}

#[test]
fn palette_constants_match_prototype() {
    let c = json("constants.json");
    let strs = |v: &Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(strs(&c["CLIFF_ROWS"]), CLIFF_ROWS);
    assert_eq!(strs(&c["FOAM_ROWS"]), FOAM_ROWS);
    assert_eq!(strs(&c["GRASS_BANDS"]), GRASS_BANDS);
    assert_eq!(strs(&c["MUD_BANDS"]), MUD_BANDS);
    assert_eq!(strs(&c["SEA_BANDS"]), SEA_BANDS);
    assert_eq!(c["AIR_COLOR"], AIR_COLOR);
    for (crop, want) in FIELD_CROPS.iter().zip(c["FIELD_CROPS"].as_array().unwrap()) {
        assert_eq!(want["key"], crop.key);
        assert_eq!(want["base"], crop.base);
        assert_eq!(want["furrow"], crop.furrow);
        assert_eq!(want["edge"], crop.edge);
        assert_eq!(want["h"].as_f64().unwrap(), crop.h);
        assert_eq!(want["w"].as_f64().unwrap(), crop.w);
    }
    for (name, value) in [
        ("CLIFF_W", 24.0),
        ("CLIFF_H", 4.0),
        ("BEACH_PX", 3.0),
        ("FOAM_PX", 7.0),
        ("FOAM_FRAMES", 8.0),
        ("FURROW_PX", 5.0),
    ] {
        assert_eq!(c[name].as_f64().unwrap(), value, "{name}");
    }
}
