//! The single-line level string (`docs/design/level-format.md`): parse, validate, serialise, and
//! build the starting world.
#![forbid(unsafe_code)]

use std::fmt;

use cl_hexsphere::HexSphere;
use cl_model::world::{frequency_is_valid, tile_count};
use cl_model::{Cover, Faction, Terrain, UnitKind, UnitView, WorldSnapshot};
use cl_worldgen::{generate_terrain_with_fraction, seed_cover};
use serde::{Deserialize, Serialize};

/// Format tag every level starts with.
pub const FORMAT_VERSION: &str = "CL1";

/// Default land share in percent (the prototype's `LAND_FRACTION`).
pub const DEFAULT_LAND_PERCENT: u8 = 42;
/// Default forest growth chance per forest tile per turn, in percent.
pub const DEFAULT_FOREST_PERCENT: u8 = 10;
/// Default AI hostility (0–100).
pub const DEFAULT_HOSTILITY: u8 = 50;
/// Default AI intelligence tier (0–3).
pub const DEFAULT_INTELLIGENCE: u8 = 1;

/// One AI player's profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSpec {
    /// Which side the AI plays.
    pub faction: Faction,
    /// 0 = passive, 100 = attacks at every chance.
    pub hostility: u8,
    /// 0 = random legal moves, 3 = strongest tier.
    pub intelligence: u8,
}

/// How a match is won, beyond eliminating every rival capital.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Victory {
    /// Own at least `percent` of the land for `turns` consecutive own turns.
    Majority {
        /// Land share in percent.
        percent: u8,
        /// Consecutive turns the share must hold.
        turns: u8,
    },
}

impl Default for Victory {
    fn default() -> Self {
        Victory::Majority {
            percent: 60,
            turns: 3,
        }
    }
}

/// A change applied to tiles `from..=to` after generation. `None` fields keep the generated value;
/// `owner: Some(None)` clears the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileDiff {
    /// First tile id.
    pub from: u32,
    /// Last tile id, inclusive.
    pub to: u32,
    /// `L` / `S`.
    pub terrain: Option<Terrain>,
    /// `E` / `F` / `O` / `T` / `C`.
    pub cover: Option<Cover>,
    /// `R` / `Y` / `G` / `B`, or `N` for none.
    pub owner: Option<Option<Faction>>,
    /// `p` / `w` / `k`.
    pub unit: Option<UnitKind>,
}

/// A parsed level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    /// Optional label, `[A-Za-z0-9_.-]+`.
    pub name: Option<String>,
    /// Hex-sphere frequency, `2..=12`.
    pub frequency: u8,
    /// World seed; `0` means no generated continents (all sea until `tiles` says otherwise).
    pub seed: u32,
    /// Number of factions in play, `2..=4`, taken in turn order from `Red`.
    pub players: u8,
    /// Factions controlled by people; the rest are AI.
    pub humans: Vec<Faction>,
    /// Explicit AI profiles; unlisted AI factions use the defaults.
    pub ai: Vec<AiSpec>,
    /// Land share in percent for generation.
    pub land_percent: u8,
    /// Forest growth chance in percent per forest tile per turn.
    pub forest_percent: u8,
    /// Victory rule.
    pub victory: Victory,
    /// Tile diffs applied in order after generation.
    pub tiles: Vec<TileDiff>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            name: None,
            frequency: 8,
            seed: 1,
            players: 2,
            humans: vec![Faction::Red],
            ai: Vec::new(),
            land_percent: DEFAULT_LAND_PERCENT,
            forest_percent: DEFAULT_FOREST_PERCENT,
            victory: Victory::default(),
            tiles: Vec::new(),
        }
    }
}

/// Why a level string was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LevelError {
    /// The string does not start with [`FORMAT_VERSION`].
    #[error("level must start with {FORMAT_VERSION}, found {0:?}")]
    Version(String),
    /// A field without `=` or with an empty key.
    #[error("malformed field {0:?}")]
    Field(String),
    /// A key the format does not define.
    #[error("unknown key {0:?}")]
    UnknownKey(String),
    /// A key given twice.
    #[error("duplicate key {0:?}")]
    DuplicateKey(String),
    /// A value that does not parse or is out of range.
    #[error("{key}={value:?}: {reason}")]
    Value {
        /// Field key.
        key: String,
        /// Offending value.
        value: String,
        /// What was expected.
        reason: String,
    },
    /// A tile entry that does not parse.
    #[error("tile entry {0:?}: {1}")]
    Tile(String, String),
    /// A required key is absent.
    #[error("missing key {0:?}")]
    Missing(&'static str),
    /// A cross-field rule failed (see [`Level::validate`]).
    #[error("{0}")]
    Invalid(String),
}

fn value_err(key: &str, value: &str, reason: impl Into<String>) -> LevelError {
    LevelError::Value {
        key: key.to_owned(),
        value: value.to_owned(),
        reason: reason.into(),
    }
}

fn parse_u8(key: &str, value: &str, range: std::ops::RangeInclusive<u8>) -> Result<u8, LevelError> {
    let v: u8 = value
        .parse()
        .map_err(|_| value_err(key, value, "expected a small integer"))?;
    if !range.contains(&v) {
        return Err(value_err(
            key,
            value,
            format!("expected {}..={}", range.start(), range.end()),
        ));
    }
    Ok(v)
}

fn parse_factions(key: &str, value: &str) -> Result<Vec<Faction>, LevelError> {
    let mut out = Vec::new();
    for c in value.chars() {
        let f = Faction::from_letter(c)
            .ok_or_else(|| value_err(key, value, "expected letters from RYGB"))?;
        if out.contains(&f) {
            return Err(value_err(key, value, format!("faction {c} listed twice")));
        }
        out.push(f);
    }
    if out.is_empty() {
        return Err(value_err(key, value, "expected at least one faction"));
    }
    Ok(out)
}

fn parse_ai(value: &str) -> Result<AiSpec, LevelError> {
    let key = "ai";
    let (f, rest) = value
        .split_once(':')
        .ok_or_else(|| value_err(key, value, "expected F:h<0-100>,i<0-3>"))?;
    let mut chars = f.chars();
    let faction = match (chars.next(), chars.next()) {
        (Some(c), None) => Faction::from_letter(c),
        _ => None,
    }
    .ok_or_else(|| value_err(key, value, "expected one faction letter before ':'"))?;
    let mut spec = AiSpec {
        faction,
        hostility: DEFAULT_HOSTILITY,
        intelligence: DEFAULT_INTELLIGENCE,
    };
    for part in rest.split(',').filter(|p| !p.is_empty()) {
        match part.split_at(1) {
            ("h", n) => {
                spec.hostility = parse_u8(key, n, 0..=100)
                    .map_err(|_| value_err(key, value, "h expects 0..=100"))?
            }
            ("i", n) => {
                spec.intelligence =
                    parse_u8(key, n, 0..=3).map_err(|_| value_err(key, value, "i expects 0..=3"))?
            }
            _ => {
                return Err(value_err(
                    key,
                    value,
                    format!("unknown AI property {part:?}"),
                ));
            }
        }
    }
    Ok(spec)
}

fn parse_victory(value: &str) -> Result<Victory, LevelError> {
    let key = "victory";
    let (kind, args) = value.split_once(':').unwrap_or((value, ""));
    match kind {
        "majority" => {
            let (p, t) = args
                .split_once(',')
                .ok_or_else(|| value_err(key, value, "expected majority:<percent>,<turns>"))?;
            Ok(Victory::Majority {
                percent: parse_u8(key, p, 1..=100)?,
                turns: parse_u8(key, t, 1..=100)?,
            })
        }
        _ => Err(value_err(key, value, "unknown victory rule")),
    }
}

fn parse_tile(entry: &str) -> Result<TileDiff, LevelError> {
    let err = |reason: &str| LevelError::Tile(entry.to_owned(), reason.to_owned());
    let digits_end = entry
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(entry.len());
    let (range, letters) = entry.split_at(digits_end);
    let (from, to) = match range.split_once('-') {
        Some((a, b)) => (
            a.parse::<u32>().map_err(|_| err("bad range start"))?,
            b.parse::<u32>().map_err(|_| err("bad range end"))?,
        ),
        None => {
            let id = range
                .parse::<u32>()
                .map_err(|_| err("expected a tile id"))?;
            (id, id)
        }
    };
    if from > to {
        return Err(err("range start after end"));
    }
    let mut diff = TileDiff {
        from,
        to,
        terrain: None,
        cover: None,
        owner: None,
        unit: None,
    };
    for c in letters.chars() {
        if let Some(t) = Terrain::from_letter(c) {
            if diff.terrain.replace(t).is_some() {
                return Err(err("terrain given twice"));
            }
        } else if let Some(cv) = Cover::from_letter(c) {
            if diff.cover.replace(cv).is_some() {
                return Err(err("cover given twice"));
            }
        } else if c == 'N' || Faction::from_letter(c).is_some() {
            if diff.owner.replace(Faction::from_letter(c)).is_some() {
                return Err(err("owner given twice"));
            }
        } else if let Some(u) = UnitKind::from_letter(c) {
            if diff.unit.replace(u).is_some() {
                return Err(err("unit given twice"));
            }
        } else {
            return Err(err(&format!("unknown letter {c:?}")));
        }
    }
    if letters.is_empty() {
        return Err(err("entry changes nothing"));
    }
    Ok(diff)
}

impl Level {
    /// Parses one level line and validates it.
    pub fn parse(text: &str) -> Result<Level, LevelError> {
        let text = text.trim();
        let mut fields = text.split(';');
        let version = fields.next().unwrap_or_default();
        if version != FORMAT_VERSION {
            return Err(LevelError::Version(version.to_owned()));
        }
        let mut level = Level::default();
        let mut seen: Vec<String> = Vec::new();
        let mut has_n = false;
        for field in fields {
            if field.is_empty() {
                continue;
            }
            let (key, value) = field
                .split_once('=')
                .ok_or_else(|| LevelError::Field(field.to_owned()))?;
            if key.is_empty() {
                return Err(LevelError::Field(field.to_owned()));
            }
            if key != "ai" {
                if seen.iter().any(|k| k == key) {
                    return Err(LevelError::DuplicateKey(key.to_owned()));
                }
                seen.push(key.to_owned());
            }
            match key {
                "name" => {
                    if value.is_empty()
                        || !value
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
                    {
                        return Err(value_err(key, value, "expected [A-Za-z0-9_.-]+"));
                    }
                    level.name = Some(value.to_owned());
                }
                "n" => {
                    level.frequency = parse_u8(key, value, 2..=12)?;
                    has_n = true;
                }
                "seed" => {
                    level.seed = value
                        .parse()
                        .map_err(|_| value_err(key, value, "expected 0..=4294967295"))?
                }
                "players" => level.players = parse_u8(key, value, 2..=4)?,
                "human" => level.humans = parse_factions(key, value)?,
                "ai" => level.ai.push(parse_ai(value)?),
                "land" => level.land_percent = parse_u8(key, value, 0..=100)?,
                "forest" => level.forest_percent = parse_u8(key, value, 0..=100)?,
                "victory" => level.victory = parse_victory(value)?,
                "tiles" => {
                    for entry in value.split(',').filter(|e| !e.is_empty()) {
                        level.tiles.push(parse_tile(entry)?);
                    }
                }
                _ => return Err(LevelError::UnknownKey(key.to_owned())),
            }
        }
        if !has_n {
            return Err(LevelError::Missing("n"));
        }
        level.validate()?;
        Ok(level)
    }

    /// Cross-field rules: factions in play, no faction both human and AI, tile ids in range.
    pub fn validate(&self) -> Result<(), LevelError> {
        let invalid = |m: String| LevelError::Invalid(m);
        if !frequency_is_valid(self.frequency) {
            return Err(invalid(format!("n={} outside 2..=12", self.frequency)));
        }
        if !(2..=4).contains(&self.players) {
            return Err(invalid(format!("players={} outside 2..=4", self.players)));
        }
        let in_play = |f: Faction| f.index() < usize::from(self.players);
        if self.humans.is_empty() {
            return Err(invalid("at least one human faction".into()));
        }
        for f in &self.humans {
            if !in_play(*f) {
                return Err(invalid(format!(
                    "human {} is not among the first {} factions",
                    f.letter(),
                    self.players
                )));
            }
        }
        for (i, a) in self.ai.iter().enumerate() {
            if !in_play(a.faction) {
                return Err(invalid(format!(
                    "ai {} is not among the first {} factions",
                    a.faction.letter(),
                    self.players
                )));
            }
            if self.humans.contains(&a.faction) {
                return Err(invalid(format!(
                    "faction {} is both human and ai",
                    a.faction.letter()
                )));
            }
            if self.ai[..i].iter().any(|b| b.faction == a.faction) {
                return Err(invalid(format!("ai {} listed twice", a.faction.letter())));
            }
            if a.hostility > 100 || a.intelligence > 3 {
                return Err(invalid(format!("ai {} out of range", a.faction.letter())));
            }
        }
        if self.land_percent > 100 || self.forest_percent > 100 {
            return Err(invalid("percentages must be 0..=100".into()));
        }
        let count = tile_count(self.frequency) as u32;
        for d in &self.tiles {
            if d.from > d.to || d.to >= count {
                return Err(invalid(format!(
                    "tile range {}-{} outside 0..{count}",
                    d.from, d.to
                )));
            }
            if let Some(f) = d.owner.flatten()
                && !in_play(f)
            {
                return Err(invalid(format!("tile owner {} is not in play", f.letter())));
            }
        }
        Ok(())
    }

    /// Factions in play, in turn order.
    pub fn factions(&self) -> Vec<Faction> {
        Faction::ALL[..usize::from(self.players)].to_vec()
    }

    /// AI profile for a faction: the explicit one, or defaults for any non-human faction in play.
    pub fn ai_for(&self, f: Faction) -> Option<AiSpec> {
        if self.humans.contains(&f) || f.index() >= usize::from(self.players) {
            return None;
        }
        Some(
            self.ai
                .iter()
                .copied()
                .find(|a| a.faction == f)
                .unwrap_or(AiSpec {
                    faction: f,
                    hostility: DEFAULT_HOSTILITY,
                    intelligence: DEFAULT_INTELLIGENCE,
                }),
        )
    }

    /// The starting world: generated terrain and cover (none when `seed == 0`), then every diff.
    /// Fails when the diffs produce an impossible tile, such as a capital without an owner.
    pub fn to_snapshot(&self) -> Result<WorldSnapshot, LevelError> {
        let sphere = HexSphere::build(self.frequency);
        let mut world = WorldSnapshot::new(self.frequency, self.seed);
        if self.seed != 0 {
            let levels = generate_terrain_with_fraction(
                &sphere,
                f64::from(self.seed),
                f64::from(self.land_percent) / 100.0,
            );
            let cover = seed_cover(&sphere, &levels, f64::from(self.seed));
            for (t, (&level, cover)) in world.tiles.iter_mut().zip(levels.iter().zip(cover)) {
                t.terrain = Terrain::from_level(level);
                t.cover = cover;
            }
        }
        for d in &self.tiles {
            for id in d.from..=d.to {
                let t = &mut world.tiles[id as usize];
                if let Some(terrain) = d.terrain {
                    t.terrain = terrain;
                    if terrain == Terrain::Sea {
                        t.cover = Cover::None;
                        t.owner = None;
                        t.unit = None;
                    }
                }
                if let Some(cover) = d.cover {
                    t.cover = cover;
                }
                if let Some(owner) = d.owner {
                    t.owner = owner;
                }
                if let Some(kind) = d.unit {
                    t.unit = t.owner.map(|owner| UnitView {
                        kind,
                        owner,
                        can_act: false,
                    });
                }
            }
        }
        world.validate().map_err(LevelError::Invalid)?;
        Ok(world)
    }
}

impl fmt::Display for TileDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.from == self.to {
            write!(f, "{}", self.from)?
        } else {
            write!(f, "{}-{}", self.from, self.to)?
        }
        if let Some(t) = self.terrain {
            write!(f, "{}", t.letter())?;
        }
        if let Some(c) = self.cover {
            write!(f, "{}", c.letter())?;
        }
        if let Some(o) = self.owner {
            write!(f, "{}", o.map_or('N', Faction::letter))?;
        }
        if let Some(u) = self.unit {
            write!(f, "{}", u.letter())?;
        }
        Ok(())
    }
}

impl fmt::Display for Level {
    /// Canonical form: every scalar field, `ai` entries in the given order, `tiles` last.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{FORMAT_VERSION}")?;
        if let Some(name) = &self.name {
            write!(f, ";name={name}")?;
        }
        write!(
            f,
            ";n={};seed={};players={}",
            self.frequency, self.seed, self.players
        )?;
        write!(
            f,
            ";human={}",
            self.humans.iter().map(|h| h.letter()).collect::<String>()
        )?;
        for a in &self.ai {
            write!(
                f,
                ";ai={}:h{},i{}",
                a.faction.letter(),
                a.hostility,
                a.intelligence
            )?;
        }
        write!(
            f,
            ";land={};forest={}",
            self.land_percent, self.forest_percent
        )?;
        match self.victory {
            Victory::Majority { percent, turns } => {
                write!(f, ";victory=majority:{percent},{turns}")?
            }
        }
        if !self.tiles.is_empty() {
            write!(f, ";tiles=")?;
            for (i, d) in self.tiles.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "{d}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    const SAMPLE: &str = "CL1;n=8;seed=1234;players=3;human=R;ai=Y:h60,i2;ai=G:h30,i1;land=42;forest=10;victory=majority:60,3;tiles=12LTRp,13LFR,14LCY,40S,77-90L";

    #[test]
    fn sample_round_trips() {
        let level = Level::parse(SAMPLE).unwrap();
        assert_eq!(level.frequency, 8);
        assert_eq!(level.players, 3);
        assert_eq!(level.ai.len(), 2);
        assert_eq!(level.tiles.len(), 5);
        assert_eq!(
            level.tiles[0],
            TileDiff {
                from: 12,
                to: 12,
                terrain: Some(Terrain::Land),
                cover: Some(Cover::Town),
                owner: Some(Some(Faction::Red)),
                unit: Some(UnitKind::Pawn)
            }
        );
        assert_eq!(
            level.tiles[4],
            TileDiff {
                from: 77,
                to: 90,
                terrain: Some(Terrain::Land),
                cover: None,
                owner: None,
                unit: None
            }
        );
        assert_eq!(level.to_string(), SAMPLE);
        assert_eq!(Level::parse(&level.to_string()).unwrap(), level);
    }

    #[test]
    fn errors_name_the_problem() {
        assert!(matches!(
            Level::parse("CL0;n=8"),
            Err(LevelError::Version(_))
        ));
        assert!(matches!(
            Level::parse("CL1;seed=1"),
            Err(LevelError::Missing("n"))
        ));
        assert!(matches!(
            Level::parse("CL1;n=8;bogus=1"),
            Err(LevelError::UnknownKey(_))
        ));
        assert!(matches!(
            Level::parse("CL1;n=8;n=9"),
            Err(LevelError::DuplicateKey(_))
        ));
        assert!(matches!(
            Level::parse("CL1;n=13"),
            Err(LevelError::Value { .. })
        ));
        assert!(matches!(
            Level::parse("CL1;n=8;tiles=12LL"),
            Err(LevelError::Tile(..))
        ));
        assert!(matches!(
            Level::parse("CL1;n=8;tiles=9999L"),
            Err(LevelError::Invalid(_))
        ));
        assert!(matches!(
            Level::parse("CL1;n=8;human=R;ai=R:h1,i1"),
            Err(LevelError::Invalid(_))
        ));
        assert!(matches!(
            Level::parse("CL1;n=8;players=2;human=G"),
            Err(LevelError::Invalid(_))
        ));
    }

    #[test]
    fn seed_zero_is_all_sea_until_tiles_say_otherwise() {
        let level = Level::parse("CL1;n=2;seed=0;tiles=0-5L,3TR,4CYp,2p").unwrap();
        let w = level.to_snapshot().unwrap();
        assert_eq!(w.tiles.iter().filter(|t| t.terrain.is_land()).count(), 6);
        assert_eq!(w.tiles[3].cover, Cover::Town);
        assert_eq!(w.tiles[3].owner, Some(Faction::Red));
        assert_eq!(w.tiles[4].cover, Cover::Capital);
        assert_eq!(w.tiles[4].unit.unwrap().owner, Faction::Yellow);
        assert!(w.tiles[2].unit.is_none(), "a unit needs an owner");
        let bad = Level::parse("CL1;n=2;seed=0;tiles=0L,0C").unwrap();
        assert!(matches!(bad.to_snapshot(), Err(LevelError::Invalid(m)) if m.contains("capital")));
    }

    #[test]
    fn generated_world_is_valid_and_ai_defaults_fill_in() {
        let level = Level::parse("CL1;n=4;seed=31676;players=4;human=RB").unwrap();
        assert!(level.to_snapshot().is_ok());
        assert_eq!(level.ai_for(Faction::Red), None);
        assert_eq!(
            level.ai_for(Faction::Yellow).unwrap().hostility,
            DEFAULT_HOSTILITY
        );
        assert_eq!(level.factions().len(), 4);
    }
}
