use serde::{Deserialize, Serialize};

/// One of the four playable sides. Declaration order is turn order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Faction {
    /// Plays first.
    Red,
    /// Plays second.
    Yellow,
    /// Plays third.
    Green,
    /// Plays last.
    Blue,
}

impl Faction {
    /// All factions in turn order.
    pub const ALL: [Faction; 4] = [Faction::Red, Faction::Yellow, Faction::Green, Faction::Blue];

    /// Zero-based slot, also the turn order.
    pub fn index(self) -> usize {
        match self {
            Faction::Red => 0,
            Faction::Yellow => 1,
            Faction::Green => 2,
            Faction::Blue => 3,
        }
    }

    /// Faction by slot; `None` outside `0..4`.
    pub fn from_index(i: usize) -> Option<Faction> {
        Faction::ALL.get(i).copied()
    }

    /// Single-letter code used by the level string (`R`, `Y`, `G`, `B`).
    pub fn letter(self) -> char {
        match self {
            Faction::Red => 'R',
            Faction::Yellow => 'Y',
            Faction::Green => 'G',
            Faction::Blue => 'B',
        }
    }

    /// Inverse of [`Faction::letter`].
    pub fn from_letter(c: char) -> Option<Faction> {
        match c {
            'R' => Some(Faction::Red),
            'Y' => Some(Faction::Yellow),
            'G' => Some(Faction::Green),
            'B' => Some(Faction::Blue),
            _ => None,
        }
    }

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Faction::Red => "Red",
            Faction::Yellow => "Yellow",
            Faction::Green => "Green",
            Faction::Blue => "Blue",
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_indices_round_trip() {
        for (i, f) in Faction::ALL.iter().enumerate() {
            assert_eq!(f.index(), i);
            assert_eq!(Faction::from_index(i), Some(*f));
            assert_eq!(Faction::from_letter(f.letter()), Some(*f));
        }
        assert_eq!(Faction::from_index(4), None);
        assert_eq!(Faction::from_letter('N'), None);
    }
}
