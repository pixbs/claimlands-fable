//! The game state: tiles, treasuries, settings, statistics.

use std::collections::BTreeMap;

use cl_model::{
    Board, Cover, Faction, Terrain, TileId, TileState, UnitKind, UnitView, WorldSnapshot,
};
use cl_noise::Mulberry32;
use serde::{Deserialize, Serialize};

/// A unit standing on a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    /// Class.
    pub kind: UnitKind,
    /// Faction that owns it.
    pub owner: Faction,
    /// Turn number it was created on (R-UNIT-03 starves the oldest first).
    pub born: u32,
    /// `true` once it has moved or upgraded this turn.
    pub acted: bool,
}

/// One tile of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    /// Land or sea.
    pub terrain: Terrain,
    /// What stands on the tile.
    pub cover: Cover,
    /// Faction whose territory includes the tile.
    pub owner: Option<Faction>,
    /// Unit on the tile.
    pub unit: Option<Unit>,
    /// Turn number the capital on this tile was founded on, if it is a capital (R-CAP-04 ties).
    pub capital_born: Option<u32>,
}

impl Tile {
    /// `true` when land with no unit.
    pub fn is_free_land(&self) -> bool {
        self.terrain.is_land() && self.unit.is_none()
    }
}

/// Gold and wheat of one territory, kept on its capital tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Treasury {
    /// Gold.
    pub gold: i64,
    /// Wheat.
    pub wheat: i64,
}

/// How a match is won, beyond eliminating every rival capital (R-VIC-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VictoryRule {
    /// R-VIC-02: own at least `percent` of the land for `turns` consecutive own turns.
    Majority {
        /// Land share in percent.
        percent: u8,
        /// Consecutive turns the share must hold.
        turns: u8,
    },
}

/// Match settings that rules read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Factions in play, in turn order.
    pub factions: Vec<Faction>,
    /// R-FOR-03: chance in percent that a forest tile seeds an adjacent empty tile each turn.
    pub forest_percent: u8,
    /// Victory rule.
    pub victory: VictoryRule,
    /// Starting treasury of every capital.
    pub starting_treasury: Treasury,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            factions: vec![Faction::Red, Faction::Yellow],
            forest_percent: 10,
            victory: VictoryRule::Majority {
                percent: 60,
                turns: 3,
            },
            starting_treasury: Treasury { gold: 5, wheat: 5 },
        }
    }
}

/// Running totals shown on the victory screen (R-STAT-01).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Stats {
    /// Completed full rounds.
    pub turns: u32,
    /// Units killed by each faction.
    pub killed: BTreeMap<Faction, u32>,
    /// Units each faction lost, to combat or starvation.
    pub lost: BTreeMap<Faction, u32>,
    /// Gold each faction has earned over the match.
    pub all_time_gold: BTreeMap<Faction, i64>,
}

/// A connected group of same-owner tiles with its capital and treasury (R-CAP-*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Territory {
    /// Owner.
    pub owner: Faction,
    /// Capital tile, if the group currently has one.
    pub capital: Option<TileId>,
    /// Member tiles in ascending id order.
    pub tiles: Vec<TileId>,
}

impl Territory {
    /// Tiles carrying `cover`.
    pub fn count_cover(&self, state: &GameState, cover: Cover) -> usize {
        self.tiles
            .iter()
            .filter(|t| state.tile(**t).cover == cover)
            .count()
    }

    /// Units of the owner standing in the territory, filtered by kind.
    pub fn count_units(&self, state: &GameState, keep: impl Fn(UnitKind) -> bool) -> usize {
        self.tiles
            .iter()
            .filter(|t| state.tile(**t).unit.is_some_and(|u| keep(u.kind)))
            .count()
    }
}

/// The complete state of a match. Change it only through [`crate::apply`].
#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    /// Full rounds completed; starts at 0.
    pub turn: u32,
    /// Faction whose turn it is.
    pub current: Faction,
    /// Tiles indexed by id.
    pub tiles: Vec<Tile>,
    /// Treasury per capital tile.
    pub treasuries: BTreeMap<TileId, Treasury>,
    /// Match settings.
    pub settings: Settings,
    /// Running totals.
    pub stats: Stats,
    /// Set once a faction has won; no command but none is accepted afterwards.
    pub winner: Option<Faction>,
    /// Gameplay randomness (forest growth, capital relocation ties).
    pub rng: Mulberry32,
    /// Consecutive own turns the current leader has held the majority (R-VIC-02).
    pub majority_streak: BTreeMap<Faction, u8>,
}

impl GameState {
    /// Starts a match from a snapshot: tiles copy over, every `Capital` receives the starting
    /// treasury, and `Red` (or the first faction in play) moves first.
    pub fn new(world: &WorldSnapshot, settings: Settings, seed: u32) -> Self {
        let tiles: Vec<Tile> = world
            .tiles
            .iter()
            .map(|t| Tile {
                terrain: t.terrain,
                cover: t.cover,
                owner: t.owner,
                unit: t.unit.map(|u| Unit {
                    kind: u.kind,
                    owner: u.owner,
                    born: 0,
                    acted: false,
                }),
                capital_born: (t.cover == Cover::Capital).then_some(0),
            })
            .collect();
        let treasuries = tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| t.cover == Cover::Capital)
            .map(|(i, _)| (TileId(i as u32), settings.starting_treasury))
            .collect();
        let current = settings.factions.first().copied().unwrap_or(Faction::Red);
        Self {
            turn: 0,
            current,
            tiles,
            treasuries,
            settings,
            stats: Stats::default(),
            winner: None,
            rng: Mulberry32::new(f64::from(seed)),
            majority_streak: BTreeMap::new(),
        }
    }

    /// Tile by id.
    pub fn tile(&self, id: TileId) -> &Tile {
        &self.tiles[id.index()]
    }

    /// Mutable tile by id.
    pub fn tile_mut(&mut self, id: TileId) -> &mut Tile {
        &mut self.tiles[id.index()]
    }

    /// Every tile id in order.
    pub fn ids(&self) -> impl Iterator<Item = TileId> + '_ {
        (0..self.tiles.len()).map(|i| TileId(i as u32))
    }

    /// Land tiles.
    pub fn land_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.terrain.is_land()).count()
    }

    /// Land tiles owned by `faction`.
    pub fn owned_count(&self, faction: Faction) -> usize {
        self.tiles
            .iter()
            .filter(|t| t.owner == Some(faction))
            .count()
    }

    /// Factions that still own a capital, in turn order.
    pub fn living_factions(&self) -> Vec<Faction> {
        self.settings
            .factions
            .iter()
            .copied()
            .filter(|f| {
                self.tiles
                    .iter()
                    .any(|t| t.cover == Cover::Capital && t.owner == Some(*f))
            })
            .collect()
    }

    /// What the view needs; `frequency` and `seed` come from the level.
    pub fn snapshot(&self, frequency: u8, seed: u32) -> WorldSnapshot {
        let can_build: BTreeMap<TileId, bool> = self
            .treasuries
            .iter()
            .map(|(&id, t)| {
                (
                    id,
                    t.gold
                        >= crate::constants::FIELD_BASE_COST.min(crate::constants::PAWN_BASE_COST),
                )
            })
            .collect();
        WorldSnapshot {
            frequency,
            seed,
            tiles: self
                .ids()
                .map(|id| {
                    let t = self.tile(id);
                    TileState {
                        terrain: t.terrain,
                        cover: t.cover,
                        owner: t.owner,
                        unit: t.unit.map(|u| UnitView {
                            kind: u.kind,
                            owner: u.owner,
                            can_act: !u.acted && u.owner == self.current,
                        }),
                        can_build: t.cover == Cover::Capital
                            && t.owner == Some(self.current)
                            && can_build.get(&id).copied().unwrap_or(false),
                    }
                })
                .collect(),
        }
    }
}

/// Connected same-owner components over `board`, each with its capital if it has one. Tiles
/// are visited in id order and neighbours in board order, so the result is deterministic.
pub fn territories(state: &GameState, board: &dyn Board) -> Vec<Territory> {
    let n = state.tiles.len();
    debug_assert_eq!(
        n,
        board.tile_count(),
        "state and board disagree on tile count"
    );
    let mut seen = vec![false; n];
    let mut out = Vec::new();
    for start in 0..n {
        let Some(owner) = state.tiles[start].owner else {
            continue;
        };
        if seen[start] {
            continue;
        }
        let mut tiles = Vec::new();
        let mut queue = vec![TileId(start as u32)];
        seen[start] = true;
        while let Some(id) = queue.pop() {
            tiles.push(id);
            for &j in board.neighbors(id) {
                if !seen[j.index()] && state.tiles[j.index()].owner == Some(owner) {
                    seen[j.index()] = true;
                    queue.push(j);
                }
            }
        }
        tiles.sort_unstable();
        let capital = tiles
            .iter()
            .copied()
            .find(|t| state.tile(*t).cover == Cover::Capital);
        out.push(Territory {
            owner,
            capital,
            tiles,
        });
    }
    out
}

/// The territory containing `id`, if the tile is owned.
pub fn territory_of(state: &GameState, board: &dyn Board, id: TileId) -> Option<Territory> {
    territories(state, board)
        .into_iter()
        .find(|t| t.tiles.binary_search(&id).is_ok())
}
