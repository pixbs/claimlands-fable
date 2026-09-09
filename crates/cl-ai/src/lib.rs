//! AI players: a `Profile` selects a `Policy`; a policy returns the next legal `Command`.
#![forbid(unsafe_code)]

use cl_model::Board;
use cl_noise::Mulberry32;
use cl_rules::{Command, GameState};
use serde::{Deserialize, Serialize};

/// Difficulty knobs, as the level string defines them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// 0 = never attacks, 100 = attacks at every chance.
    pub hostility: u8,
    /// 0 = passes, 1 = random legal, 2 = greedy, 3 = lookahead (tiers land with M5 issues).
    pub intelligence: u8,
}

/// Picks commands for one faction.
pub trait Policy {
    /// The next command for the current faction; `Command::EndTurn` when done.
    fn choose(&mut self, state: &GameState, board: &dyn Board) -> Command;
}

/// Tier 0: ends the turn immediately. Exists so a match with AI players can run end to end
/// before smarter tiers exist.
#[derive(Debug, Clone)]
pub struct Passive {
    /// Kept so tier upgrades keep the same constructor shape.
    #[allow(dead_code)]
    rng: Mulberry32,
    /// Profile the policy was built from.
    pub profile: Profile,
}

impl Policy for Passive {
    fn choose(&mut self, _state: &GameState, _board: &dyn Board) -> Command {
        Command::EndTurn
    }
}

/// The policy for a profile. Every tier currently maps to [`Passive`]; M5 issues replace the
/// higher tiers one by one.
pub fn policy_for(profile: Profile, seed: u32) -> Box<dyn Policy> {
    Box::new(Passive {
        rng: Mulberry32::new(f64::from(seed)),
        profile,
    })
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_model::{GraphBoard, TileState, WorldSnapshot};
    use cl_rules::{Settings, apply};

    use super::*;

    #[test]
    fn passive_policy_only_emits_legal_commands() {
        let board = GraphBoard::flower();
        let world = WorldSnapshot {
            frequency: 2,
            seed: 0,
            tiles: vec![TileState::land(); 7],
        };
        let mut state = GameState::new(&world, Settings::default(), 1);
        let mut policy = policy_for(
            Profile {
                hostility: 50,
                intelligence: 0,
            },
            3,
        );
        for _ in 0..4 {
            let cmd = policy.choose(&state, &board);
            assert!(
                apply(&mut state, &board, cmd).is_ok(),
                "{cmd:?} must be legal"
            );
        }
        assert_eq!(state.turn, 2);
    }
}
