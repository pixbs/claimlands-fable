# Rules

Every rule has an id. Tests carry the id in their names; issues cite it; `crates/cl-rules/src/constants.rs`
names each number after it. Status: **done** (implemented and tested), **open** (backlog issue), or
**decided** (a default chosen for a gap in the original spec; a *Design question* issue tracks it).

## Board and factions

| Id | Rule | Status |
|---|---|---|
| R-TURN-01 | Turn order is Red, Yellow, Green, Blue among factions that still own a capital. | done |
| R-TURN-02 | `EndTurn` pays the ending faction's income (R-ECO), then applies upkeep (R-UNIT-02/03), town feeding (R-ECO-04), forest growth (R-FOR-03) and the victory check (R-VIC), then passes the turn; a full round increments the turn counter. | income done, rest open |
| R-TURN-03 | Any command of the current turn can be undone until `EndTurn`; undo replays from the turn start (`cl-session`). | done |
| R-TURN-04 | The camera of a watching human follows the acting faction's moves. | open (app) |

## Economy (per territory, paid into its capital's treasury)

| Id | Rule | Status |
|---|---|---|
| R-ECO-01 | Owned empty tile: +1 wheat per turn. | done |
| R-ECO-02 | Capital: +1 wheat, +1 gold per turn. | done |
| R-ECO-03 | Field: +2 wheat per turn. | done |
| R-ECO-04 | Town: +2 gold, −3 wheat per turn. Towns are fed whole, in tile-id order, after units, until the wheat runs out; an unfed town yields no gold. Example: 3 towns and 7 wheat → 2 towns fed, 4 gold, 6 wheat eaten. | open |
| R-ECO-05 | Wheat feeds units before towns. | open |
| R-ECO-06 | Forest: nothing. | done (no income branch) |

## Building (within the acting faction's territory, paid by that territory)

| Id | Rule | Status |
|---|---|---|
| R-BLD-01 | Town on an owned empty tile: 2 gold + 1 per town already in the territory. | done |
| R-BLD-02 | Field on an owned empty tile: 1 gold + 1 per field already in the territory. | done |
| R-BLD-03 | A town or field entered by an enemy unit becomes empty. | open |

## Forest

| Id | Rule | Status |
|---|---|---|
| R-FOR-01 | A forest tile yields nothing and cannot be built on. | done |
| R-FOR-02 | A unit entering a forest tile clears it to empty. | open |
| R-FOR-03 | Each turn, each forest tile has an `N %` chance (level default 10) to seed one random adjacent empty tile, owned or not; never fields, towns or capitals. | open, decided (owned tiles included) |

## Units

| Id | Rule | Status |
|---|---|---|
| R-UNIT-01 | Recruit a pawn on an owned land tile without a unit: 1 gold + 1 per unit already in the territory (any kind). | done |
| R-UNIT-02 | Upkeep per turn: pawn 1 wheat; warrior 2 wheat; knight 2 wheat + 1 gold. | open |
| R-UNIT-03 | When a territory cannot pay upkeep, units die until it can: knights first, then warriors, then pawns; oldest first within a kind. | open, decided (order) |
| R-UNIT-04 | One move per turn: up to 4 steps through the unit's own territory plus 1 step into any other tile, which becomes owned. A unit cannot end on a tile with another unit. | open |
| R-UNIT-05 | A pawn may enter empty, forest and field tiles only. | open |
| R-UNIT-06 | A warrior may also enter enemy towns and capitals and take enemy pawns; never a warrior or knight, never a tile with an allied unit. | open |
| R-UNIT-07 | A knight may take any enemy unit. | open |
| R-UNIT-08 | Upgrade pawn → warrior: 1 gold + 1 per warrior or knight in the territory; needs and consumes the unit's move. | done |
| R-UNIT-09 | Upgrade warrior → knight: 2 gold + 2 per knight in the territory; needs and consumes the move. | done |
| R-UNIT-10 | Entering a tile claims it for the unit's owner (R-BLD-03, R-FOR-02, R-CAP-01 apply). | open |

## Capitals and territories

| Id | Rule | Status |
|---|---|---|
| R-CAP-01 | An enemy unit entering a capital turns it into an empty tile; the attacker's territory receives 25 % of its gold (floor). | open |
| R-CAP-02 | Every territory has exactly one capital. When one is lost, a new capital appears on the empty tile closest to the territory's centre (least maximal graph distance, ties by seeded RNG); with no empty tile, a town; with none, a field; with none, the territory is dissolved and its tiles become unowned. | open, decided (centre metric) |
| R-CAP-03 | When a territory is split, the part without a capital gets one by R-CAP-02, and gold and wheat are divided in proportion to tile counts: the new part gets `floor(amount × newTiles / totalTiles)`, the old part keeps the rest. Example: 16 tiles, 15 gold, 15 wheat, split 10/5 → new part 5/5, old part 10/10. | open, decided (rounding) |
| R-CAP-04 | When two territories of one faction merge, the capital closest to the captured tile stays; the other becomes empty. Tie: the capital founded earlier stays; if founded the same turn, the larger territory's; if equal, the lower tile id. | open, decided (tie-break) |
| R-CAP-05 | Merged territories pool their treasuries. | open |

## Victory and statistics

| Id | Rule | Status |
|---|---|---|
| R-VIC-01 | A faction wins when no rival owns a capital. | open |
| R-VIC-02 | A faction wins when it has owned at least `percent` of the land for `turns` consecutive own turns (level default 60 %, 3 turns). | open, decided (defaults) |
| R-STAT-01 | Track turns, units killed and lost per faction, all-time gold per faction; the victory screen shows them. | partially done (gold, turns) |
| R-STAT-02 | A match is a level string plus its command log; replaying it reproduces the final state without AI. | done |

## Availability indicators

| Id | Rule | Status |
|---|---|---|
| R-UI-01 | A unit with a move or upgrade left shows a rotating star. | `can_act` in snapshot done; star mesh open |
| R-UI-02 | A capital whose territory can afford any action shows the same star. | `can_build` done (cheapest action); star mesh open |
