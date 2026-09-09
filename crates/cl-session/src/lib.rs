//! One running match: players, turn flow, undo within a turn, command log and replay.
#![forbid(unsafe_code)]

use cl_ai::{Policy, Profile, policy_for};
use cl_hexsphere::HexSphere;
use cl_level::{Level, LevelError, Victory};
use cl_model::{Faction, WorldSnapshot};
use cl_rules::{Command, Event, GameState, RuleError, Settings, VictoryRule, apply};
use serde::{Deserialize, Serialize};

/// Who controls a faction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    /// A person at the screen.
    Human,
    /// A computer opponent with this profile.
    Ai(Profile),
}

/// Everything needed to reproduce a match: the level line and every accepted command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    /// Level as a line.
    pub level: String,
    /// Accepted commands with the faction that issued them.
    pub commands: Vec<(Faction, Command)>,
}

/// Why a session could not start or replay.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    /// The level line is invalid.
    #[error("level: {0}")]
    Level(#[from] LevelError),
    /// A recorded command was refused on replay.
    #[error("replay: command {index} ({command:?}) refused: {error}")]
    Replay {
        /// Position in the recording.
        index: usize,
        /// The command.
        command: Command,
        /// Why the rules refused it.
        error: RuleError,
    },
}

/// A running match.
pub struct Session {
    level: Level,
    sphere: HexSphere,
    state: GameState,
    turn_start: GameState,
    since_turn_start: Vec<Command>,
    log: Vec<(Faction, Command)>,
    policies: Vec<(Faction, Box<dyn Policy>)>,
}

impl Session {
    /// Starts a match from a level.
    pub fn start(level: Level) -> Result<Session, SessionError> {
        let world = level.to_snapshot()?;
        let sphere = HexSphere::build(level.frequency);
        let settings = Settings {
            factions: level.factions(),
            forest_percent: level.forest_percent,
            victory: match level.victory {
                Victory::Majority { percent, turns } => VictoryRule::Majority { percent, turns },
            },
            ..Settings::default()
        };
        let state = GameState::new(&world, settings, level.seed);
        let policies = level
            .factions()
            .into_iter()
            .filter_map(|f| level.ai_for(f).map(|a| (f, a)))
            .map(|(f, a)| {
                let profile = Profile {
                    hostility: a.hostility,
                    intelligence: a.intelligence,
                };
                (
                    f,
                    policy_for(profile, level.seed.wrapping_add(f.index() as u32)),
                )
            })
            .collect();
        Ok(Session {
            turn_start: state.clone(),
            state,
            level,
            sphere,
            since_turn_start: Vec::new(),
            log: Vec::new(),
            policies,
        })
    }

    /// Replays a recording; fails on the first refused command.
    pub fn replay(recording: &Recording) -> Result<Session, SessionError> {
        let level = Level::parse(&recording.level)?;
        let mut session = Session::start(level)?;
        for (index, &(faction, command)) in recording.commands.iter().enumerate() {
            if session.state.current != faction {
                return Err(SessionError::Replay {
                    index,
                    command,
                    error: RuleError::NotYetImplemented("R-TURN-01"),
                });
            }
            session
                .apply(command)
                .map_err(|error| SessionError::Replay {
                    index,
                    command,
                    error,
                })?;
        }
        Ok(session)
    }

    /// The level the match runs on.
    pub fn level(&self) -> &Level {
        &self.level
    }

    /// Current rules state.
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// The board.
    pub fn board(&self) -> &HexSphere {
        &self.sphere
    }

    /// Who controls the faction whose turn it is.
    pub fn current_player(&self) -> PlayerKind {
        self.level
            .ai_for(self.state.current)
            .map_or(PlayerKind::Human, |a| {
                PlayerKind::Ai(Profile {
                    hostility: a.hostility,
                    intelligence: a.intelligence,
                })
            })
    }

    /// Applies a command for the current faction and logs it when accepted.
    pub fn apply(&mut self, command: Command) -> Result<Vec<Event>, RuleError> {
        let faction = self.state.current;
        let events = apply(&mut self.state, &self.sphere, command)?;
        self.log.push((faction, command));
        if command == Command::EndTurn {
            self.turn_start = self.state.clone();
            self.since_turn_start.clear();
        } else {
            self.since_turn_start.push(command);
        }
        Ok(events)
    }

    /// Undoes the last command of the current turn; `None` when the turn has no commands yet.
    pub fn undo(&mut self) -> Option<Command> {
        let undone = self.since_turn_start.pop()?;
        self.log.pop();
        self.state = self.turn_start.clone();
        for &cmd in &self.since_turn_start {
            apply(&mut self.state, &self.sphere, cmd).expect("previously accepted commands replay");
        }
        Some(undone)
    }

    /// Lets the current AI faction play its whole turn. Returns `None` when a human is up.
    pub fn play_ai(&mut self) -> Option<Vec<Event>> {
        let faction = self.state.current;
        let idx = self.policies.iter().position(|(f, _)| *f == faction)?;
        let mut events = Vec::new();
        for _ in 0..1000 {
            let cmd = self.policies[idx].1.choose(&self.state, &self.sphere);
            let ended = cmd == Command::EndTurn;
            match self.apply(cmd) {
                Ok(e) => events.extend(e),
                Err(_) => {
                    events.extend(
                        self.apply(Command::EndTurn)
                            .expect("ending a turn is always legal"),
                    );
                    break;
                }
            }
            if ended {
                break;
            }
        }
        Some(events)
    }

    /// What the view reads.
    pub fn snapshot(&self) -> WorldSnapshot {
        self.state.snapshot(self.level.frequency, self.level.seed)
    }

    /// The match so far.
    pub fn recording(&self) -> Recording {
        Recording {
            level: self.level.to_string(),
            commands: self.log.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use cl_model::TileId;

    use super::*;

    fn tiny() -> Session {
        // Red: capital 0 plus tiles 1-3; Yellow (AI): capital 5 plus 4 and 6. Frequency 2 (42 tiles).
        let level = Level::parse(
            "CL1;n=2;seed=0;players=2;human=R;ai=Y:h0,i0;tiles=0-6L,0-3N,0CR,1-3ER,5CY,4EY,6EY",
        )
        .unwrap();
        Session::start(level).unwrap()
    }

    #[test]
    fn undo_restores_the_previous_state() {
        let mut s = tiny();
        let before = s.state().clone();
        s.apply(Command::BuildField { at: TileId(1) }).unwrap();
        assert_ne!(s.state(), &before);
        assert_eq!(s.undo(), Some(Command::BuildField { at: TileId(1) }));
        assert_eq!(s.state(), &before);
        assert_eq!(s.undo(), None);
    }

    #[test]
    fn replay_reproduces_the_match() {
        let mut s = tiny();
        s.apply(Command::BuildField { at: TileId(1) }).unwrap();
        s.apply(Command::EndTurn).unwrap();
        assert_eq!(
            s.current_player(),
            PlayerKind::Ai(Profile {
                hostility: 0,
                intelligence: 0
            })
        );
        s.play_ai().unwrap();
        assert_eq!(s.state().current, Faction::Red);
        let rec = s.recording();
        let json = serde_json::to_string(&rec).unwrap();
        let back: Recording = serde_json::from_str(&json).unwrap();
        let replayed = Session::replay(&back).unwrap();
        assert_eq!(replayed.state(), s.state());
        assert_eq!(replayed.snapshot(), s.snapshot());
    }
}
