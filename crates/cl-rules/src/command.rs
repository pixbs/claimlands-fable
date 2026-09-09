//! Commands a player issues, events a command produces, and the reasons a command is refused.

use cl_model::{Cover, Faction, TileId, UnitKind};
use serde::{Deserialize, Serialize};

/// What a player can do on their turn. Every variant is replayable: the command log plus the
/// level string reproduce a match exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Move the unit on `from` to `to` (R-UNIT-04..07, R-UNIT-10).
    MoveUnit {
        /// Tile the unit stands on.
        from: TileId,
        /// Destination tile.
        to: TileId,
    },
    /// Create a pawn on an owned, free land tile (R-UNIT-01).
    RecruitPawn {
        /// Where the pawn appears.
        at: TileId,
    },
    /// Upgrade the unit on `at` one step: pawn → warrior → knight (R-UNIT-08, R-UNIT-09).
    Upgrade {
        /// Tile of the unit.
        at: TileId,
    },
    /// Build a town on an owned empty tile (R-BLD-01).
    BuildTown {
        /// Target tile.
        at: TileId,
    },
    /// Build a field on an owned empty tile (R-BLD-02).
    BuildField {
        /// Target tile.
        at: TileId,
    },
    /// Finish the turn (R-TURN-02): income, upkeep, growth, victory check, next faction.
    EndTurn,
}

/// What changed. Emitted in the order the changes happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// A faction's turn began.
    TurnStarted {
        /// Faction to move.
        faction: Faction,
        /// Full rounds completed so far.
        turn: u32,
    },
    /// A territory's treasury changed by income or upkeep.
    TreasuryChanged {
        /// Capital of the territory.
        capital: TileId,
        /// Gold delta.
        gold: i64,
        /// Wheat delta.
        wheat: i64,
    },
    /// A tile changed owner.
    OwnerChanged {
        /// Tile.
        tile: TileId,
        /// Previous owner.
        from: Option<Faction>,
        /// New owner.
        to: Option<Faction>,
    },
    /// A tile's cover changed (built, destroyed, grown, cleared, capital moved).
    CoverChanged {
        /// Tile.
        tile: TileId,
        /// Previous cover.
        from: Cover,
        /// New cover.
        to: Cover,
    },
    /// A unit was created.
    UnitCreated {
        /// Where.
        tile: TileId,
        /// Class.
        kind: UnitKind,
    },
    /// A unit changed class.
    UnitUpgraded {
        /// Where.
        tile: TileId,
        /// New class.
        kind: UnitKind,
    },
    /// A unit moved.
    UnitMoved {
        /// Origin.
        from: TileId,
        /// Destination.
        to: TileId,
    },
    /// A unit left the board.
    UnitDied {
        /// Where it stood.
        tile: TileId,
        /// Its class.
        kind: UnitKind,
        /// Why.
        cause: DeathCause,
    },
    /// A capital was founded or moved.
    CapitalMoved {
        /// Previous capital tile, if any.
        from: Option<TileId>,
        /// New capital tile.
        to: TileId,
    },
    /// A faction won.
    Victory {
        /// Winner.
        faction: Faction,
    },
}

/// Why a unit died.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathCause {
    /// Taken by an enemy unit.
    Captured,
    /// The territory could not feed it (R-UNIT-03).
    Starved,
}

/// Why a command was refused. The state is unchanged when one is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum RuleError {
    /// The match is over.
    #[error("the match is over")]
    GameOver,
    /// The tile id is beyond the board.
    #[error("no such tile {0}")]
    NoSuchTile(TileId),
    /// Water.
    #[error("{0} is sea")]
    NotLand(TileId),
    /// The tile is not in the current faction's territory.
    #[error("{0} is not in your territory")]
    NotOwned(TileId),
    /// The territory has no capital to pay from.
    #[error("{0} belongs to a territory without a capital")]
    NoCapital(TileId),
    /// Not enough gold.
    #[error("{needed} gold needed, {available} available")]
    CannotAfford {
        /// Cost of the action.
        needed: i64,
        /// Gold in the territory.
        available: i64,
    },
    /// The tile must be empty (bare land) for this action.
    #[error("{0} is not an empty tile")]
    NotEmpty(TileId),
    /// A unit already stands there.
    #[error("{0} is occupied")]
    Occupied(TileId),
    /// No unit of the current faction stands there.
    #[error("no unit of yours on {0}")]
    NoUnit(TileId),
    /// The unit has already moved or upgraded this turn.
    #[error("the unit on {0} has already acted")]
    AlreadyActed(TileId),
    /// Knights cannot be upgraded further.
    #[error("the unit on {0} is already a knight")]
    MaxRank(TileId),
    /// The destination cannot be reached under R-UNIT-04.
    #[error("{to} is out of range from {from}")]
    OutOfRange {
        /// Origin.
        from: TileId,
        /// Destination.
        to: TileId,
    },
    /// The unit class may not enter that tile (R-UNIT-05..07).
    #[error("the unit on {from} may not enter {to}")]
    Forbidden {
        /// Origin.
        from: TileId,
        /// Destination.
        to: TileId,
    },
    /// The rule that would handle this command is still on the backlog.
    #[error("rule {0} is not implemented yet")]
    NotYetImplemented(&'static str),
}
