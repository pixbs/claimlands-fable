use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Faction;
use crate::world::tile_count;

/// Index of a tile on the hex sphere. Ids follow the geodesic build order and are stable for a given
/// frequency; `fixtures/hexsphere` pins them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TileId(pub u32);

impl TileId {
    /// The id as a `Vec` index.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for TileId {
    fn from(v: u32) -> Self {
        TileId(v)
    }
}

impl fmt::Display for TileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Elevation class of a tile. Level 0 is the land shell, level -1 the sea shell four texture
/// pixels below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Terrain {
    /// Claimable, buildable ground.
    Land,
    /// Water: nothing stands or moves here.
    #[default]
    Sea,
}

impl Terrain {
    /// Integer elevation level as the prototype stores it.
    pub fn level(self) -> i32 {
        match self {
            Terrain::Land => 0,
            Terrain::Sea => -1,
        }
    }

    /// Inverse of [`Terrain::level`]: anything at or above 0 is land.
    pub fn from_level(level: i32) -> Terrain {
        if level >= 0 {
            Terrain::Land
        } else {
            Terrain::Sea
        }
    }

    /// Level-string code (`L` or `S`).
    pub fn letter(self) -> char {
        match self {
            Terrain::Land => 'L',
            Terrain::Sea => 'S',
        }
    }

    /// Inverse of [`Terrain::letter`].
    pub fn from_letter(c: char) -> Option<Terrain> {
        match c {
            'L' => Some(Terrain::Land),
            'S' => Some(Terrain::Sea),
            _ => None,
        }
    }

    /// `true` for [`Terrain::Land`].
    pub fn is_land(self) -> bool {
        self == Terrain::Land
    }
}

/// What stands on a land tile. One per tile, mutually exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Cover {
    /// Bare grass: +1 wheat per turn when owned.
    #[default]
    None,
    /// Farmland: +2 wheat per turn.
    Field,
    /// Woods: yields nothing, spreads onto empty neighbours, cleared when a pawn enters.
    Forest,
    /// Village: +2 gold, -3 wheat per turn.
    Town,
    /// Seat of a territory: +1 wheat, +1 gold per turn and the territory's treasury.
    Capital,
}

impl Cover {
    /// Level-string code (`E`, `F`, `O`, `T`, `C`).
    pub fn letter(self) -> char {
        match self {
            Cover::None => 'E',
            Cover::Field => 'F',
            Cover::Forest => 'O',
            Cover::Town => 'T',
            Cover::Capital => 'C',
        }
    }

    /// Inverse of [`Cover::letter`].
    pub fn from_letter(c: char) -> Option<Cover> {
        match c {
            'E' => Some(Cover::None),
            'F' => Some(Cover::Field),
            'O' => Some(Cover::Forest),
            'T' => Some(Cover::Town),
            'C' => Some(Cover::Capital),
            _ => None,
        }
    }

    /// `Town` or `Capital`: the covers an enemy unit destroys or takes over.
    pub fn is_building(self) -> bool {
        matches!(self, Cover::Town | Cover::Capital)
    }
}

/// Unit class; each upgrade is a strict superset of the previous one's moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    /// Claims empty, forest and field tiles.
    Pawn,
    /// Also takes towns, capitals and enemy pawns.
    Warrior,
    /// Also takes enemy warriors and knights.
    Knight,
}

impl UnitKind {
    /// Level-string code (`p`, `w`, `k`).
    pub fn letter(self) -> char {
        match self {
            UnitKind::Pawn => 'p',
            UnitKind::Warrior => 'w',
            UnitKind::Knight => 'k',
        }
    }

    /// Inverse of [`UnitKind::letter`].
    pub fn from_letter(c: char) -> Option<UnitKind> {
        match c {
            'p' => Some(UnitKind::Pawn),
            'w' => Some(UnitKind::Warrior),
            'k' => Some(UnitKind::Knight),
            _ => None,
        }
    }
}

/// A unit as the view needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitView {
    /// Class of the unit.
    pub kind: UnitKind,
    /// Faction that owns it.
    pub owner: Faction,
    /// `true` while the unit still has a move or upgrade this turn (the rotating star).
    pub can_act: bool,
}

/// Everything the view needs to know about one tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TileState {
    /// Land or sea.
    pub terrain: Terrain,
    /// What stands on the tile.
    pub cover: Cover,
    /// Faction whose territory includes the tile.
    pub owner: Option<Faction>,
    /// Unit standing on the tile.
    pub unit: Option<UnitView>,
    /// `true` on a capital whose territory can afford something (the capital's rotating star).
    pub can_build: bool,
}

impl TileState {
    /// Bare, unowned land.
    pub fn land() -> Self {
        Self {
            terrain: Terrain::Land,
            ..Self::default()
        }
    }

    /// Checks the cross-field rules; `Err` names the first violation.
    pub fn validate(&self) -> Result<(), String> {
        if self.terrain == Terrain::Sea {
            if self.cover != Cover::None {
                return Err(format!("sea tile carries cover {:?}", self.cover));
            }
            if self.owner.is_some() {
                return Err("sea tile has an owner".into());
            }
            if self.unit.is_some() {
                return Err("sea tile has a unit".into());
            }
        }
        if self.cover == Cover::Capital && self.owner.is_none() {
            return Err("capital without an owner".into());
        }
        if self.can_build && self.cover != Cover::Capital {
            return Err("can_build set on a non-capital".into());
        }
        Ok(())
    }
}

/// Frequency, seed and one [`TileState`] per tile: the complete input of the view layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// Hex-sphere frequency `n`; the sphere has `10n² + 2` tiles.
    pub frequency: u8,
    /// Seed the world was generated from (0 = no generated continents).
    pub seed: u32,
    /// One entry per tile, indexed by [`TileId`].
    pub tiles: Vec<TileState>,
}

impl WorldSnapshot {
    /// All-sea snapshot of the right size.
    pub fn new(frequency: u8, seed: u32) -> Self {
        Self {
            frequency,
            seed,
            tiles: vec![TileState::default(); tile_count(frequency)],
        }
    }

    /// State of one tile.
    pub fn tile(&self, id: TileId) -> &TileState {
        &self.tiles[id.index()]
    }

    /// Mutable state of one tile.
    pub fn tile_mut(&mut self, id: TileId) -> &mut TileState {
        &mut self.tiles[id.index()]
    }

    /// Ids of every tile, in order.
    pub fn ids(&self) -> impl Iterator<Item = TileId> + '_ {
        (0..self.tiles.len()).map(|i| TileId(i as u32))
    }

    /// Elevation level per tile, as the geometry builders take it.
    pub fn levels(&self) -> Vec<i32> {
        self.tiles.iter().map(|t| t.terrain.level()).collect()
    }

    /// Size and per-tile rules; `Err` names the first violation.
    pub fn validate(&self) -> Result<(), String> {
        let expected = tile_count(self.frequency);
        if self.tiles.len() != expected {
            return Err(format!(
                "frequency {} needs {expected} tiles, snapshot has {}",
                self.frequency,
                self.tiles.len()
            ));
        }
        for (i, t) in self.tiles.iter().enumerate() {
            t.validate().map_err(|e| format!("tile #{i}: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn letters_round_trip() {
        for c in [
            Cover::None,
            Cover::Field,
            Cover::Forest,
            Cover::Town,
            Cover::Capital,
        ] {
            assert_eq!(Cover::from_letter(c.letter()), Some(c));
        }
        for u in [UnitKind::Pawn, UnitKind::Warrior, UnitKind::Knight] {
            assert_eq!(UnitKind::from_letter(u.letter()), Some(u));
        }
        for t in [Terrain::Land, Terrain::Sea] {
            assert_eq!(Terrain::from_letter(t.letter()), Some(t));
            assert_eq!(Terrain::from_level(t.level()), t);
        }
    }

    #[test]
    fn snapshot_validation_catches_size_and_sea_rules() {
        let mut w = WorldSnapshot::new(2, 0);
        assert_eq!(w.tiles.len(), 42);
        assert!(w.validate().is_ok());
        w.tile_mut(TileId(3)).cover = Cover::Forest;
        assert!(w.validate().unwrap_err().contains("#3"));
        let mut land = TileState::land();
        land.cover = Cover::Town;
        assert!(land.validate().is_ok(), "neutral villages are allowed");
        land.cover = Cover::Capital;
        assert!(land.validate().is_err());
        land.owner = Some(Faction::Green);
        assert!(land.validate().is_ok());
        w.tiles.pop();
        assert!(w.validate().unwrap_err().contains("needs 42"));
    }
}
