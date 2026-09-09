//! Every tunable number of the rules, named after the rule id that uses it
//! (`docs/design/rules.md`).

/// R-ECO-01: wheat per turn from an owned empty tile.
pub const EMPTY_WHEAT: i64 = 1;
/// R-ECO-02: wheat per turn from a capital.
pub const CAPITAL_WHEAT: i64 = 1;
/// R-ECO-02: gold per turn from a capital.
pub const CAPITAL_GOLD: i64 = 1;
/// R-ECO-03: wheat per turn from a field.
pub const FIELD_WHEAT: i64 = 2;
/// R-ECO-04: gold per turn from a fed town.
pub const TOWN_GOLD: i64 = 2;
/// R-ECO-04: wheat a town eats per turn.
pub const TOWN_WHEAT: i64 = 3;

/// R-BLD-01: gold for the first town in a territory; each existing town adds one.
pub const TOWN_BASE_COST: i64 = 2;
/// R-BLD-02: gold for the first field in a territory; each existing field adds one.
pub const FIELD_BASE_COST: i64 = 1;

/// R-UNIT-01: gold for a pawn; each unit already in the territory adds one.
pub const PAWN_BASE_COST: i64 = 1;
/// R-UNIT-08: gold for the warrior upgrade; each warrior or knight in the territory adds one.
pub const WARRIOR_UPGRADE_BASE_COST: i64 = 1;
/// R-UNIT-09: gold for the knight upgrade; each knight in the territory adds two.
pub const KNIGHT_UPGRADE_BASE_COST: i64 = 2;
/// R-UNIT-09: gold added per existing knight.
pub const KNIGHT_UPGRADE_STEP: i64 = 2;

/// R-UNIT-02: wheat a pawn eats per turn.
pub const PAWN_UPKEEP_WHEAT: i64 = 1;
/// R-UNIT-02: wheat a warrior eats per turn.
pub const WARRIOR_UPKEEP_WHEAT: i64 = 2;
/// R-UNIT-02: wheat a knight eats per turn.
pub const KNIGHT_UPKEEP_WHEAT: i64 = 2;
/// R-UNIT-02: gold a knight costs per turn.
pub const KNIGHT_UPKEEP_GOLD: i64 = 1;

/// R-UNIT-04: steps a unit may take inside its own territory per turn.
pub const MOVE_RANGE_OWN: u32 = 4;
/// R-UNIT-04: steps a unit may take beyond its territory (claiming the tile).
pub const MOVE_RANGE_CLAIM: u32 = 1;

/// R-CAP-01: share of the captured capital's gold the attacker receives, in percent.
pub const CAPTURE_GOLD_PERCENT: i64 = 25;
