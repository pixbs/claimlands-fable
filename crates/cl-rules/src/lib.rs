//! The game as a pure state machine. `apply` validates a `Command` against a `GameState` on a
//! `Board`, mutates the state only when every precondition holds, and returns the `Event`s that
//! describe the change. Rule ids (`R-ECO-03`) refer to `docs/design/rules.md`.
#![forbid(unsafe_code)]

mod apply;
mod command;
pub mod constants;
mod state;

pub use apply::apply;
pub use command::{Command, DeathCause, Event, RuleError};
pub use state::{
    GameState, Settings, Stats, Territory, Tile, Treasury, Unit, VictoryRule, territories,
    territory_of,
};

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_model::{
        Cover, Faction, GraphBoard, Terrain, TileId, TileState, UnitKind, WorldSnapshot,
    };

    use super::*;

    /// Seven land tiles: Red owns the centre (capital) and ring tiles 1–3, Yellow owns 4–6 with a
    /// capital on 5.
    fn two_player_flower() -> (GraphBoard, GameState) {
        let board = GraphBoard::flower();
        let mut world = WorldSnapshot {
            frequency: 2,
            seed: 0,
            tiles: vec![TileState::land(); 7],
        };
        for i in 0..=3 {
            world.tiles[i].owner = Some(Faction::Red);
        }
        for i in 4..=6 {
            world.tiles[i].owner = Some(Faction::Yellow);
        }
        world.tiles[0].cover = Cover::Capital;
        world.tiles[5].cover = Cover::Capital;
        let state = GameState::new(&world, Settings::default(), 7);
        (board, state)
    }

    #[test]
    fn territories_follow_ownership() {
        let (board, state) = two_player_flower();
        let t = territories(&state, &board);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].owner, Faction::Red);
        assert_eq!(t[0].capital, Some(TileId(0)));
        assert_eq!(t[0].tiles, vec![TileId(0), TileId(1), TileId(2), TileId(3)]);
        assert_eq!(t[1].capital, Some(TileId(5)));
    }

    #[test]
    fn r_eco_01_02_03_end_turn_income() {
        let (board, mut state) = two_player_flower();
        state.tile_mut(TileId(1)).cover = Cover::Field;
        let events = apply(&mut state, &board, Command::EndTurn).unwrap();
        // capital +1/+1, field +2 wheat, two empty tiles +1 wheat each
        assert!(events.contains(&Event::TreasuryChanged {
            capital: TileId(0),
            gold: 1,
            wheat: 5
        }));
        assert_eq!(
            state.treasuries[&TileId(0)],
            Treasury { gold: 6, wheat: 10 }
        );
        assert_eq!(
            state.treasuries[&TileId(5)],
            Treasury { gold: 5, wheat: 5 },
            "yellow earns on its own turn"
        );
        assert_eq!(state.current, Faction::Yellow);
        assert_eq!(state.turn, 0);
        apply(&mut state, &board, Command::EndTurn).unwrap();
        assert_eq!((state.current, state.turn), (Faction::Red, 1));
    }

    #[test]
    fn r_bld_02_field_cost_grows_per_field_in_territory() {
        let (board, mut state) = two_player_flower();
        assert!(apply(&mut state, &board, Command::BuildField { at: TileId(1) }).is_ok());
        assert!(apply(&mut state, &board, Command::BuildField { at: TileId(2) }).is_ok());
        assert_eq!(state.treasuries[&TileId(0)].gold, 5 - 1 - 2);
        let before = state.clone();
        assert_eq!(
            apply(&mut state, &board, Command::BuildField { at: TileId(1) }),
            Err(RuleError::NotEmpty(TileId(1)))
        );
        assert_eq!(
            apply(&mut state, &board, Command::BuildField { at: TileId(4) }),
            Err(RuleError::NotOwned(TileId(4)))
        );
        assert_eq!(state, before, "refused commands leave the state untouched");
    }

    #[test]
    fn r_bld_01_town_needs_two_gold_plus_towns() {
        let (board, mut state) = two_player_flower();
        assert!(apply(&mut state, &board, Command::BuildTown { at: TileId(1) }).is_ok());
        assert!(apply(&mut state, &board, Command::BuildTown { at: TileId(2) }).is_ok());
        assert_eq!(state.treasuries[&TileId(0)].gold, 0);
        assert_eq!(
            apply(&mut state, &board, Command::BuildTown { at: TileId(3) }),
            Err(RuleError::CannotAfford {
                needed: 4,
                available: 0
            })
        );
    }

    #[test]
    fn r_unit_01_08_09_recruit_and_upgrade_costs() {
        let (board, mut state) = two_player_flower();
        state.treasuries.get_mut(&TileId(0)).unwrap().gold = 20;
        apply(&mut state, &board, Command::RecruitPawn { at: TileId(1) }).unwrap();
        apply(&mut state, &board, Command::RecruitPawn { at: TileId(2) }).unwrap();
        assert_eq!(state.treasuries[&TileId(0)].gold, 20 - 1 - 2);
        assert_eq!(
            apply(&mut state, &board, Command::RecruitPawn { at: TileId(2) }),
            Err(RuleError::Occupied(TileId(2)))
        );
        apply(&mut state, &board, Command::Upgrade { at: TileId(1) }).unwrap();
        assert_eq!(state.tile(TileId(1)).unit.unwrap().kind, UnitKind::Warrior);
        assert_eq!(state.treasuries[&TileId(0)].gold, 17 - 1);
        assert_eq!(
            apply(&mut state, &board, Command::Upgrade { at: TileId(1) }),
            Err(RuleError::AlreadyActed(TileId(1)))
        );
        apply(&mut state, &board, Command::Upgrade { at: TileId(2) }).unwrap();
        assert_eq!(
            state.treasuries[&TileId(0)].gold,
            16 - 2,
            "second warrior costs 1 + 1 veteran"
        );
        apply(&mut state, &board, Command::EndTurn).unwrap();
        apply(&mut state, &board, Command::EndTurn).unwrap();
        apply(&mut state, &board, Command::Upgrade { at: TileId(1) }).unwrap();
        assert_eq!(state.tile(TileId(1)).unit.unwrap().kind, UnitKind::Knight);
    }

    #[test]
    fn sea_and_foreign_tiles_are_refused_and_game_over_blocks_everything() {
        let (board, mut state) = two_player_flower();
        state.tile_mut(TileId(3)).terrain = Terrain::Sea;
        state.tile_mut(TileId(3)).owner = None;
        assert_eq!(
            apply(&mut state, &board, Command::BuildField { at: TileId(3) }),
            Err(RuleError::NotLand(TileId(3)))
        );
        assert_eq!(
            apply(&mut state, &board, Command::BuildField { at: TileId(99) }),
            Err(RuleError::NoSuchTile(TileId(99)))
        );
        state.winner = Some(Faction::Red);
        assert_eq!(
            apply(&mut state, &board, Command::EndTurn),
            Err(RuleError::GameOver)
        );
    }

    #[test]
    fn snapshot_marks_actionable_units_and_capitals() {
        let (board, mut state) = two_player_flower();
        apply(&mut state, &board, Command::RecruitPawn { at: TileId(1) }).unwrap();
        let snap = state.snapshot(2, 0);
        assert!(
            snap.tiles.iter().all(|t| t.validate().is_ok()),
            "the flower is not a sphere, so only per-tile rules apply"
        );
        assert!(snap.tiles[1].unit.unwrap().can_act);
        assert!(snap.tiles[0].can_build);
        assert!(!snap.tiles[5].can_build, "not yellow's turn");
    }
}
