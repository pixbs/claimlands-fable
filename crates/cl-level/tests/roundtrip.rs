#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Property test: any valid `Level` serialises to a line that parses back to the same `Level`.

use cl_level::{AiSpec, Level, TileDiff, Victory};
use cl_model::world::tile_count;
use cl_model::{Cover, Faction, Terrain, UnitKind};
use proptest::prelude::*;

fn faction() -> impl Strategy<Value = Faction> {
    (0..4usize).prop_map(|i| Faction::from_index(i).unwrap())
}

fn tile_diff(max_id: u32) -> impl Strategy<Value = TileDiff> {
    (
        0..max_id,
        0..4u32,
        prop::option::of(
            prop::bool::ANY.prop_map(|b| if b { Terrain::Land } else { Terrain::Sea }),
        ),
        prop::option::of((0..5usize).prop_map(|i| {
            [
                Cover::None,
                Cover::Field,
                Cover::Forest,
                Cover::Town,
                Cover::Capital,
            ][i]
        })),
        prop::option::of(prop::option::of(faction())),
        prop::option::of(
            (0..3usize).prop_map(|i| [UnitKind::Pawn, UnitKind::Warrior, UnitKind::Knight][i]),
        ),
    )
        .prop_filter("an entry must change something", |(_, _, t, c, o, u)| {
            t.is_some() || c.is_some() || o.is_some() || u.is_some()
        })
        .prop_map(move |(from, span, terrain, cover, owner, unit)| {
            let to = (from + span).min(max_id - 1);
            TileDiff {
                from,
                to,
                terrain,
                cover,
                owner,
                unit,
            }
        })
}

fn level() -> impl Strategy<Value = Level> {
    (2..=12u8, 2..=4u8).prop_flat_map(|(n, players)| {
        let max_id = tile_count(n) as u32;
        (
            prop::option::of("[A-Za-z0-9_.-]{1,12}"),
            any::<u32>(),
            prop::collection::vec(0..players as usize, 1..=players as usize),
            prop::collection::vec(
                (0..players as usize, 0..=100u8, 0..=3u8),
                0..=players as usize,
            ),
            0..=100u8,
            0..=100u8,
            (1..=100u8, 1..=100u8),
            prop::collection::vec(tile_diff(max_id), 0..8),
        )
            .prop_map(
                move |(name, seed, humans, ai, land, forest, (percent, turns), tiles)| {
                    let mut humans: Vec<Faction> = humans
                        .into_iter()
                        .map(|i| Faction::from_index(i).unwrap())
                        .collect();
                    humans.sort();
                    humans.dedup();
                    let mut specs: Vec<AiSpec> = Vec::new();
                    for (i, h, intel) in ai {
                        let f = Faction::from_index(i).unwrap();
                        if humans.contains(&f) || specs.iter().any(|s| s.faction == f) {
                            continue;
                        }
                        specs.push(AiSpec {
                            faction: f,
                            hostility: h,
                            intelligence: intel,
                        });
                    }
                    let tiles = tiles
                        .into_iter()
                        .map(|mut d| {
                            if let Some(Some(f)) = d.owner
                                && f.index() >= usize::from(players)
                            {
                                d.owner = Some(None);
                            }
                            d
                        })
                        .collect();
                    Level {
                        name,
                        frequency: n,
                        seed,
                        players,
                        humans,
                        ai: specs,
                        land_percent: land,
                        forest_percent: forest,
                        victory: Victory::Majority { percent, turns },
                        tiles,
                    }
                },
            )
    })
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn serialise_then_parse_is_identity(level in level()) {
        prop_assert!(level.validate().is_ok(), "generator produced an invalid level: {level}");
        let text = level.to_string();
        let back = Level::parse(&text).map_err(|e| TestCaseError::fail(format!("{e} in {text}")))?;
        prop_assert_eq!(back, level);
    }

    #[test]
    fn snapshot_is_valid_or_the_error_names_a_tile(level in level()) {
        match level.to_snapshot() {
            Ok(w) => prop_assert!(w.validate().is_ok()),
            Err(cl_level::LevelError::Invalid(m)) => prop_assert!(m.starts_with("tile #"), "{m}"),
            Err(e) => prop_assert!(false, "unexpected error {e}"),
        }
    }
}
