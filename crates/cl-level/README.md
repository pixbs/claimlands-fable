# cl-level

## Purpose
Levels are one URL-safe line (`docs/design/level-format.md`): sphere size, seed, players, AI
profiles, tunables, and a diff of tiles applied over the generated world. This crate parses and
serialises that line, validates it, and turns it into the starting `WorldSnapshot`. Campaign levels,
editor output and shared custom levels all pass through here.

## Public API
| Item | Role |
|---|---|
| `Level` | Parsed level; `Level::parse(&str)`, `Display` (canonical serialisation), `validate()` |
| `Level::to_snapshot()` | Generates terrain and cover from `n`/`seed` (seed 0 = all sea), then applies `tiles`; `Err` for impossible tiles |
| `AiSpec`, `Victory`, `TileDiff` | Level fields as data |
| `LevelError` | Every parse or validation failure, with the offending key or entry |
| `FORMAT_VERSION` | `"CL1"`; unknown versions are rejected |

## Invariants
- `parse(level.to_string()) == level` for every valid `Level` (proptest).
- Unknown keys, duplicate keys, out-of-range values and tile ids beyond `10n² + 2` are errors, never
  silently ignored.
- The snapshot produced always passes `WorldSnapshot::validate()`.

## Testing
Unit tests per field, error cases per rule in `docs/design/level-format.md`, and a proptest
round trip over random levels.

## Non-goals
Game rules (owners' treasuries, unit moves), the editor UI, campaign level content.
