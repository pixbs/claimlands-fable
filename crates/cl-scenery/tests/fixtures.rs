#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Compares ported builders with `fixtures/scenery/*-sky.json`, extracted from the prototype.
//! The hash covers the little-endian bytes of the `f32` attribute arrays, so it is exact.

use std::fs;
use std::path::PathBuf;

use cl_hexsphere::{HexSphere, compute_tile_frames};
use cl_model::{Cover, Terrain, TileId, TileState, WorldSnapshot};
use cl_noise::js::cos;
use cl_pixelart::{CLOUD_DECKS, CLOUD_TEX_W, DITHER_FLOOR, build_terrain_atlas, make_cloud_sky};
use cl_scenery::{
    ARM_DIM, BUSH_CHANCE, BUSH_MARGIN_PX, CANOPY_BODY, CANOPY_ZONE_F, CANOPY_ZONES, CHIM_ODDS,
    CHIM_PX, CHIM_RISE_MAX, CHIM_RISE_MIN, CROWN_JIT, CROWN_PX, CROWN_STEP, DOOR_H, DOOR_W,
    FLICKER_LEVELS, FLICKER_MS, FLICKER_SHARE, FLOOR_GROW, FLOOR_LIFT_PX, FLOOR_R_MIN, FLOOR_SHADE,
    FOREST_SPAN, HOLE_REST_IN, HOLE_REST_OPEN, HOLE_REST_OUT, HOUSE_LEN_MAX, HOUSE_LEN_MIN,
    HOUSE_ODDS, HOUSE_OPEN, HOUSE_ROOF, HOUSE_ROT_JIT, HOUSE_SINK, HOUSE_SPAN, HOUSE_SPAN_MAX,
    HOUSE_SPAN_MIN, HOUSE_TURNS, HOUSE_WALLS, L_CHANCE, PLOT_JIT, PLOT_PX, PLUS_SHARE,
    RIDGE_CAP_PX, ROOF_LIP, ROOF_OVER, SKY_CORE, SKY_RIM, STAR_DENSITY, STAR_INSET, STAR_TONES,
    STRIPE_ODDS, STRIPE_PX, TREE_H_PX, TREE_MAX, TREE_MIN, TREE_SINK_PX, VERGE_ODDS, VERGE_PX,
    VIGOUR_F, WALL_MAX, WALL_MIN, WIN_PX, build_atmosphere, build_cloud_shell, build_clouds,
    build_fields, build_forest, build_houses, build_space, build_terrain, step_stars,
};
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
fn cloud_deck_stack_matches_prototype() {
    let f = fixture("clouds-n8.json");
    let tag = "n8-s63352";
    let sphere = HexSphere::build(8);
    let frames = compute_tile_frames(&sphere, &levels(tag));
    let clouds = build_clouds(&sphere, &frames, frames.px);

    assert_eq!(
        sphere.len() as u64,
        f["tileCount"].as_u64().unwrap(),
        "tile count"
    );
    assert_eq!(
        clouds.shell.radius,
        f["radius"].as_f64().unwrap(),
        "shell radius"
    );

    // The stack is drawn from one sky, so the atlas the fixture records is that sky's size.
    let sky = make_cloud_sky(f["seed"].as_f64().unwrap());
    let atlas = f["atlas"].as_array().unwrap();
    assert_eq!(u64::from(sky.width), atlas[0].as_u64().unwrap(), "atlas w");
    assert_eq!(u64::from(sky.height), atlas[1].as_u64().unwrap(), "atlas h");
    assert_eq!(sky.width, CLOUD_TEX_W);

    for (k, deck) in clouds.decks.iter().enumerate() {
        let want = &f["decks"][k];
        assert_eq!(
            deck.scale.to_bits(),
            want["scale"].as_f64().unwrap().to_bits(),
            "deck {k} scale"
        );
        assert_eq!(
            i64::from(deck.render_order),
            want["renderOrder"].as_i64().unwrap(),
            "deck {k} render order"
        );
        assert_eq!(want["material"]["color"], deck.tone, "deck {k} tone");
        assert_eq!(
            deck.tone, CLOUD_DECKS[k].tone,
            "deck {k} tone matches the sky"
        );
        assert_eq!(
            want["material"]["alphaTest"].as_f64().unwrap(),
            DITHER_FLOOR,
            "deck {k} alphaTest is the dither floor"
        );
        assert_eq!(want["material"]["side"], "DoubleSide", "deck {k} side");
        assert_eq!(want["material"]["transparent"], false, "deck {k}");
        assert_eq!(
            want["material"]["opacity"].as_f64().unwrap(),
            1.0,
            "deck {k}"
        );
        // The hole rests shut; its angles are the prototype's initial cap. `uFocus` is not
        // compared: the harness's `Vector3` stub drops its arguments, so the fixture records
        // (0,0,0) where the prototype passes (0,0,1).
        let u = &want["uniforms"];
        assert_eq!(
            u["uOpen"].as_f64().unwrap(),
            HOLE_REST_OPEN,
            "deck {k} hole shut"
        );
        assert_eq!(
            u["uHoleOut"].as_f64().unwrap().to_bits(),
            cos(HOLE_REST_OUT).to_bits(),
            "deck {k} hole outer angle"
        );
        assert_eq!(
            u["uHoleIn"].as_f64().unwrap().to_bits(),
            cos(HOLE_REST_IN).to_bits(),
            "deck {k} hole inner angle"
        );
    }
    // Deck 0 rides the shell itself; each one after it stands off by the same step.
    assert_eq!(clouds.decks[0].scale, 1.0);
    let step = clouds.decks[1].scale - clouds.decks[0].scale;
    assert!((clouds.decks[2].scale - clouds.decks[1].scale - step).abs() < 1e-12);
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

#[test]
fn forest_meshes_match_prototype_worlds() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let f = fixture(&format!("{tag}-cover.json"));
        let (sphere, snapshot) = world(tag);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let forest = build_forest(
            &sphere,
            &frames,
            &snapshot,
            frames.px,
            f64::from(snapshot.seed),
        )
        .expect("every prototype world carries forest");
        let want = &f["forest"];
        assert_eq!(
            forest.zones,
            want["zones"].as_u64().unwrap() as usize,
            "{tag} forest zones"
        );
        assert_eq!(
            forest.tiles,
            want["tiles"].as_u64().unwrap() as usize,
            "{tag} forest tiles"
        );
        assert_eq!(
            forest.crowns,
            want["crowns"].as_u64().unwrap() as usize,
            "{tag} forest crowns"
        );
        for (name, got) in [
            ("position", &forest.surface.positions),
            ("color", &forest.surface.colors),
            ("normal", &forest.surface.normals),
        ] {
            check_attribute(&format!("{tag} forest.{name}"), got, &want["surface"][name]);
        }
        assert!(
            forest.surface.validate().is_ok(),
            "{tag} forest: {:?}",
            forest.surface.validate()
        );
    }
}

#[test]
fn forest_constants_match_prototype() {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "fixtures"]
        .iter()
        .collect();
    let c: Value =
        serde_json::from_str(&fs::read_to_string(path.join("constants.json")).unwrap()).unwrap();
    assert_eq!(c["CANOPY"]["body"], CANOPY_BODY);
    let zones: Vec<String> = c["CANOPY"]["zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(zones, CANOPY_ZONES);
    for (name, value) in [
        ("CROWN_PX", CROWN_PX),
        ("CROWN_STEP", CROWN_STEP),
        ("CROWN_JIT", CROWN_JIT),
        ("TREE_H_PX", TREE_H_PX),
        ("TREE_SINK_PX", TREE_SINK_PX),
        ("BUSH_MARGIN_PX", BUSH_MARGIN_PX),
        ("BUSH_CHANCE", BUSH_CHANCE),
        ("CANOPY_ZONE_F", CANOPY_ZONE_F),
        ("TREE_MIN", TREE_MIN),
        ("TREE_MAX", TREE_MAX),
        ("VIGOUR_F", VIGOUR_F),
        ("FLOOR_R_MIN", FLOOR_R_MIN),
        ("FLOOR_GROW", FLOOR_GROW),
        ("FLOOR_LIFT_PX", FLOOR_LIFT_PX),
        ("FLOOR_SHADE", FLOOR_SHADE),
        ("FOREST_SPAN", FOREST_SPAN),
    ] {
        assert_eq!(c[name].as_f64().unwrap(), value, "{name}");
    }
}

#[test]
fn house_meshes_match_prototype_worlds() {
    for tag in ["n4-s31676", "n4-s1234", "n8-s63352"] {
        let f = fixture(&format!("{tag}-cover.json"));
        let (sphere, snapshot) = world(tag);
        let frames = compute_tile_frames(&sphere, &snapshot.levels());
        let houses = build_houses(
            &sphere,
            &frames,
            &snapshot,
            frames.px,
            f64::from(snapshot.seed),
        )
        .expect("every prototype world carries a village");
        let want = &f["houses"];
        for (name, got, expected) in [
            (
                "zones",
                houses.zones,
                want["zones"].as_u64().unwrap() as usize,
            ),
            (
                "tiles",
                houses.tiles,
                want["tiles"].as_u64().unwrap() as usize,
            ),
            (
                "houses",
                houses.houses,
                want["houses"].as_u64().unwrap() as usize,
            ),
            (
                "wings",
                houses.wings,
                want["wings"].as_u64().unwrap() as usize,
            ),
        ] {
            assert_eq!(got, expected, "{tag} houses {name}");
        }
        for (name, got) in [
            ("position", &houses.surface.positions),
            ("color", &houses.surface.colors),
            ("normal", &houses.surface.normals),
        ] {
            check_attribute(&format!("{tag} houses.{name}"), got, &want["surface"][name]);
        }
        assert!(
            houses.surface.validate().is_ok(),
            "{tag} houses: {:?}",
            houses.surface.validate()
        );
    }
}

#[test]
fn house_constants_match_prototype() {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "fixtures"]
        .iter()
        .collect();
    let c: Value =
        serde_json::from_str(&fs::read_to_string(path.join("constants.json")).unwrap()).unwrap();
    let strs = |v: &Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(strs(&c["HOUSE_WALLS"]), HOUSE_WALLS);
    assert_eq!(c["HOUSE_ROOF"]["a"], HOUSE_ROOF[0]);
    assert_eq!(c["HOUSE_ROOF"]["b"], HOUSE_ROOF[1]);
    assert_eq!(c["HOUSE_OPEN"], HOUSE_OPEN);
    let turns: Vec<f64> = c["HOUSE_TURNS"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(turns, HOUSE_TURNS, "HOUSE_TURNS");
    for (name, value) in [
        ("PLOT_PX", PLOT_PX),
        ("PLOT_JIT", PLOT_JIT),
        ("HOUSE_SPAN_MIN", HOUSE_SPAN_MIN),
        ("HOUSE_SPAN_MAX", HOUSE_SPAN_MAX),
        ("HOUSE_LEN_MIN", HOUSE_LEN_MIN),
        ("HOUSE_LEN_MAX", HOUSE_LEN_MAX),
        ("WALL_MIN", WALL_MIN),
        ("WALL_MAX", WALL_MAX),
        ("ROOF_OVER", ROOF_OVER),
        ("ROOF_LIP", ROOF_LIP),
        ("STRIPE_PX", STRIPE_PX),
        ("RIDGE_CAP_PX", RIDGE_CAP_PX),
        ("STRIPE_ODDS", STRIPE_ODDS),
        ("VERGE_PX", VERGE_PX),
        ("VERGE_ODDS", VERGE_ODDS),
        ("DOOR_W", DOOR_W),
        ("DOOR_H", DOOR_H),
        ("WIN_PX", WIN_PX),
        ("CHIM_PX", CHIM_PX),
        ("CHIM_RISE_MIN", CHIM_RISE_MIN),
        ("CHIM_RISE_MAX", CHIM_RISE_MAX),
        ("CHIM_ODDS", CHIM_ODDS),
        ("L_CHANCE", L_CHANCE),
        ("HOUSE_SINK", HOUSE_SINK),
        ("HOUSE_ODDS", HOUSE_ODDS),
        ("HOUSE_SPAN", HOUSE_SPAN),
        ("HOUSE_ROT_JIT", HOUSE_ROT_JIT),
    ] {
        assert_eq!(c[name].as_f64().unwrap(), value, "{name}");
    }
}

/// The raw `f32` array a full fixture dump carries, read back exactly.
fn floats(v: &Value) -> Vec<f32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap() as f32)
        .collect()
}

#[test]
fn the_space_pass_matches_prototype_sizes() {
    for (w, h) in [(320u32, 180u32), (300, 200)] {
        let tag = format!("{w}x{h}");
        let f = fixture(&format!("space-{tag}.json"));
        let space = build_space(w, h);
        assert!(space.vignette.validate().is_ok(), "{tag} vignette");
        assert!(space.stars.validate().is_ok(), "{tag} stars");

        // The prototype's vignette is an indexed plane and `MeshData` is a plain triangle list, so
        // the fixture's index is what the two are compared through.
        let grid = floats(&f["vignette"]["position"]);
        let tint = floats(&f["vignette"]["color"]);
        let index: Vec<usize> = f["vignette"]["index"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect();
        let expand = |src: &[f32]| -> Vec<f32> {
            index
                .iter()
                .flat_map(|&v| src[v * 3..v * 3 + 3].to_vec())
                .collect()
        };
        assert_eq!(
            space.vignette.positions,
            expand(&grid),
            "{tag} vignette xyz"
        );
        assert_eq!(space.vignette.colors, expand(&tint), "{tag} vignette rgb");

        assert_eq!(
            space.stars.positions,
            floats(&f["stars"]["position"]),
            "{tag} star xyz"
        );
        assert_eq!(
            space.stars.colors,
            floats(&f["stars"]["color"]),
            "{tag} star rgb"
        );

        let want = f["stars"]["list"].as_array().unwrap();
        assert_eq!(space.list.len(), want.len(), "{tag} star count");
        for (i, (got, want)) in space.list.iter().zip(want).enumerate() {
            let tone: Vec<u8> = want["tone"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u8)
                .collect();
            assert_eq!(
                (
                    got.start as u64,
                    got.count as u64,
                    got.cells as u64,
                    got.tone.to_vec(),
                    got.plus,
                    got.flick,
                    got.phase,
                    got.rate,
                ),
                (
                    want["start"].as_u64().unwrap(),
                    want["count"].as_u64().unwrap(),
                    want["cells"].as_u64().unwrap(),
                    tone,
                    want["plus"].as_bool().unwrap(),
                    want["flick"].as_bool().unwrap(),
                    want["phase"].as_f64().unwrap(),
                    want["rate"].as_f64().unwrap(),
                ),
                "{tag} star {i}"
            );
        }

        // The harness steps the same colour array twice, so a steady star still carries the colour
        // it was built with in both.
        let mut colors = space.stars.colors.clone();
        for at in ["colorAt1234", "colorAt5678"] {
            let t = at.trim_start_matches("colorAt").parse::<f64>().unwrap();
            step_stars(&mut colors, &space.list, t);
            assert_eq!(colors, floats(&f["stars"][at]), "{tag} {at}");
        }
    }
}

#[test]
fn space_constants_match_prototype() {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "fixtures"]
        .iter()
        .collect();
    let c: Value =
        serde_json::from_str(&fs::read_to_string(path.join("constants.json")).unwrap()).unwrap();
    assert_eq!(c["SKY_CORE"], SKY_CORE);
    assert_eq!(c["SKY_RIM"], SKY_RIM);
    let tones: Vec<String> = c["STAR_TONES"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(tones, STAR_TONES);
    let levels: Vec<f64> = c["FLICKER_LEVELS"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(levels, FLICKER_LEVELS, "FLICKER_LEVELS");
    for (name, value) in [
        ("STAR_DENSITY", STAR_DENSITY),
        ("PLUS_SHARE", PLUS_SHARE),
        ("ARM_DIM", ARM_DIM),
        ("STAR_INSET", STAR_INSET),
        ("FLICKER_SHARE", FLICKER_SHARE),
        ("FLICKER_MS", FLICKER_MS),
    ] {
        assert_eq!(c[name].as_f64().unwrap(), value, "{name}");
    }
}
