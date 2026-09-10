#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Compares generated textures with `fixtures/pixelart/*.png` and constants with
//! `fixtures/constants.json`, both extracted from the prototype.

use std::fs;
use std::path::PathBuf;

use cl_hexsphere::{HexSphere, compute_tile_frames};
use cl_model::{Cover, RgbaImage, Terrain, TileState, WorldSnapshot};
use cl_pixelart::palette::{
    AIR_COLOR, CLIFF_ROWS, FIELD_CROPS, FOAM_ROWS, GRASS_BANDS, MUD_BANDS, SEA_BANDS,
};
use cl_pixelart::{
    Atlas, BAYER4, CLOUD_DECKS, CLOUD_F0, CLOUD_OCT, CLOUD_TEX_W, COAST_DARKEN, COAST_TIGHT,
    DITHER_FLOOR, DITHER_RANKS, Filter, GRASS_DITHER, GRASS_F0, GRASS_OCT, MUD_DITHER, MUD_EDGE,
    MUD_F0, MUD_OCT, MUD_SALT, MUD_SCATTER, SEA_F0, SEA_FADE, SEA_OCT, SEA_SHALLOW, SPECKLE, Wrap,
    build_terrain_atlas, make_cliff_texture, make_cloud_sky, make_field_texture, make_foam_texture,
};
use serde_json::Value;

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
fn cliff_strip_matches_prototype_exactly() {
    let tex = make_cliff_texture();
    let diff = differing_texels(&tex.image, &png("pixelart/cliff.png"));
    assert!(diff.is_empty(), "cliff texels differ: {diff:?}");
    let meta = json("pixelart/cliff.json");
    assert_eq!(tex.wrap_s, wrap_of(&meta["wrapS"]));
    assert_eq!(tex.wrap_t, wrap_of(&meta["wrapT"]));
    assert_eq!(meta["magFilter"], "NearestFilter");
    assert_eq!(meta["generateMipmaps"], false);
}

#[test]
fn foam_sheet_matches_prototype_exactly() {
    let tex = make_foam_texture();
    let diff = differing_texels(&tex.image, &png("pixelart/foam.png"));
    assert!(diff.is_empty(), "foam texels differ: {diff:?}");
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

/// Seed the harness builds the sky with: `(63352 % 9973) + 7`.
const SKY_SEED: f64 = 3521.0;

#[test]
fn cloud_sky_matches_prototype_exactly() {
    let sky = make_cloud_sky(SKY_SEED);
    let meta = json("pixelart/sky-s3521.json");
    assert_eq!(meta["seed"].as_f64().unwrap(), SKY_SEED);
    assert_eq!(u64::from(sky.width), meta["W"].as_u64().unwrap(), "width");
    assert_eq!(u64::from(sky.height), meta["H"].as_u64().unwrap(), "height");
    for (k, deck) in sky.decks.iter().enumerate() {
        let want = &meta["decks"][k];
        assert_eq!(deck.wrap_s, wrap_of(&want["wrapS"]), "deck {k} wrapS");
        assert_eq!(deck.wrap_t, wrap_of(&want["wrapT"]), "deck {k} wrapT");
        assert_eq!(deck.filter, Filter::Nearest, "deck {k} filter");
        assert_eq!(want["magFilter"], "NearestFilter", "deck {k}");
        assert_eq!(want["minFilter"], "NearestFilter", "deck {k}");
        assert_eq!(want["generateMipmaps"], false, "deck {k}");
        assert_eq!(deck.repeat[0], want["repeat"]["x"].as_f64().unwrap());
        assert_eq!(deck.repeat[1], want["repeat"]["y"].as_f64().unwrap());
        assert_eq!(
            want["offset"]["x"].as_f64().unwrap(),
            0.0,
            "deck {k} offset"
        );
        assert_eq!(
            want["offset"]["y"].as_f64().unwrap(),
            0.0,
            "deck {k} offset"
        );

        let diff = differing_texels(
            &deck.image,
            &png(&format!("pixelart/sky-s3521-deck{k}.png")),
        );
        assert!(
            diff.is_empty(),
            "deck {k}: {} sky texels differ, first {:?}",
            diff.len(),
            &diff[..diff.len().min(10)]
        );
    }
}

#[test]
fn cloud_constants_match_prototype() {
    let c = json("constants.json");
    for (name, got, want) in [
        ("CLOUD_TEX_W", f64::from(CLOUD_TEX_W), &c["CLOUD_TEX_W"]),
        ("CLOUD_OCT", f64::from(CLOUD_OCT), &c["CLOUD_OCT"]),
        ("CLOUD_F0", CLOUD_F0, &c["CLOUD_F0"]),
        ("DITHER_RANKS", f64::from(DITHER_RANKS), &c["DITHER_RANKS"]),
        ("DITHER_FLOOR", DITHER_FLOOR, &c["DITHER_FLOOR"]),
    ] {
        assert_eq!(want.as_f64().unwrap(), got, "{name}");
    }
    let bayer: Vec<u64> = c["BAYER4"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    assert_eq!(
        bayer,
        BAYER4.iter().map(|&b| u64::from(b)).collect::<Vec<_>>()
    );
    for (deck, want) in CLOUD_DECKS.iter().zip(c["CLOUD_DECKS"].as_array().unwrap()) {
        assert_eq!(want["tone"], deck.tone);
        assert_eq!(want["cover"].as_f64().unwrap(), deck.cover);
    }
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

/// The prototype world behind a fixture tag, read from `fixtures/worldgen` so the atlas is compared
/// against the prototype's own levels and cover rather than against a second port.
fn world(tag: &str) -> (HexSphere, WorldSnapshot) {
    let f = json(&format!("worldgen/{tag}.json"));
    let frequency = f["n"].as_u64().unwrap() as u8;
    let seed = f["seed"].as_u64().unwrap() as u32;
    let levels = f["level"].as_array().unwrap();
    let covers = f["cover"].as_array().unwrap();
    let tiles = levels
        .iter()
        .zip(covers)
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
    let snapshot = WorldSnapshot {
        frequency,
        seed,
        tiles,
    };
    (HexSphere::build(frequency), snapshot)
}

fn atlas_of(tag: &str) -> Atlas {
    let (sphere, snapshot) = world(tag);
    let frames = compute_tile_frames(&sphere, &snapshot.levels());
    build_terrain_atlas(&sphere, &frames, &snapshot, f64::from(snapshot.seed))
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn ground_atlas_matches_prototype_worlds_exactly() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let atlas = atlas_of(tag);
        let meta = json(&format!("pixelart/atlas-{tag}.json"));
        assert_eq!(
            u64::from(atlas.cols),
            meta["cols"].as_u64().unwrap(),
            "{tag} cols"
        );
        assert_eq!(
            u64::from(atlas.rows),
            meta["rows"].as_u64().unwrap(),
            "{tag} rows"
        );
        let size = meta["size"].as_array().unwrap();
        assert_eq!(
            u64::from(atlas.texture.image.width),
            size[0].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(atlas.texture.image.height),
            size[1].as_u64().unwrap()
        );
        assert_eq!(atlas.texture.wrap_s, wrap_of(&meta["texture"]["wrapS"]));
        assert_eq!(atlas.texture.wrap_t, wrap_of(&meta["texture"]["wrapT"]));
        assert_eq!(meta["texture"]["magFilter"], "NearestFilter");
        assert_eq!(meta["texture"]["generateMipmaps"], false);

        for (i, cell) in meta["cell"].as_array().unwrap().iter().enumerate() {
            let want = [
                cell[0].as_u64().unwrap() as u32,
                cell[1].as_u64().unwrap() as u32,
            ];
            assert_eq!(atlas.cells[i], want, "{tag} cell of tile {i}");
        }

        for (name, got, want) in [
            ("prox", &atlas.fields.prox, floats(&meta["prox"])),
            ("shallow", &atlas.fields.shallow, floats(&meta["shallow"])),
            ("built", &atlas.fields.built, floats(&meta["built"])),
        ] {
            assert_eq!(got, &want, "{tag} {name}");
        }
        for (name, got, want) in [
            ("cornerProx", &atlas.fields.corner_prox, &meta["cornerProx"]),
            (
                "cornerShallow",
                &atlas.fields.corner_shallow,
                &meta["cornerShallow"],
            ),
            (
                "cornerBuilt",
                &atlas.fields.corner_built,
                &meta["cornerBuilt"],
            ),
        ] {
            let want: Vec<Vec<f64>> = want.as_array().unwrap().iter().map(floats).collect();
            assert_eq!(**got, want, "{tag} {name}");
        }

        let diff = differing_texels(
            &atlas.texture.image,
            &png(&format!("pixelart/atlas-{tag}.png")),
        );
        assert!(
            diff.is_empty(),
            "{tag}: {} atlas texels differ, first {:?}",
            diff.len(),
            &diff[..diff.len().min(10)]
        );
    }
}

#[test]
fn atlas_constants_match_prototype() {
    let c = json("constants.json");
    for (name, value) in [
        ("GRASS_OCT", f64::from(GRASS_OCT)),
        ("GRASS_F0", GRASS_F0),
        ("SEA_OCT", f64::from(SEA_OCT)),
        ("SEA_F0", SEA_F0),
        ("GRASS_DITHER", GRASS_DITHER),
        ("SPECKLE", SPECKLE),
        ("COAST_DARKEN", COAST_DARKEN),
        ("COAST_TIGHT", COAST_TIGHT),
        ("SEA_FADE", f64::from(SEA_FADE)),
        ("SEA_SHALLOW", SEA_SHALLOW),
        ("MUD_OCT", f64::from(MUD_OCT)),
        ("MUD_F0", MUD_F0),
        ("MUD_SALT", MUD_SALT),
        ("MUD_EDGE", MUD_EDGE),
        ("MUD_SCATTER", MUD_SCATTER),
        ("MUD_DITHER", MUD_DITHER),
    ] {
        assert_eq!(c[name].as_f64().unwrap(), value, "{name}");
    }
}
