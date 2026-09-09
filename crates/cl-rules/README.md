# cl-rules

## Purpose
The whole game as a pure state machine: `apply(&mut state, board, command)` either rejects the
command with a `RuleError` and leaves the state untouched, or mutates it and returns the `Event`s
that describe what changed. Nothing here knows about geometry beyond the `Board` adjacency trait,
time, threads, rendering or input, which is what makes every rule testable on a seven-tile board in
microseconds. Rule identifiers (`R-ECO-03`) in `docs/design/rules.md` name each rule; tests carry
the same identifiers.

## Public API
| Item | Role |
|---|---|
| `GameState` | Tiles, treasuries per capital, settings, stats, turn and current faction; `GameState::new(&snapshot, settings, seed)` |
| `Command` | `MoveUnit`, `RecruitPawn`, `Upgrade`, `BuildTown`, `BuildField`, `EndTurn` |
| `Event` | What a command changed, in order; the view layer marks dirty regions from these |
| `RuleError` | Why a command was refused; `NotYetImplemented(rule id)` for rules still on the backlog |
| `apply` | The only entry point that changes a state |
| `territories(&state, board)` | Connected same-owner components with their capital, recomputed on demand |
| `state.snapshot(frequency, seed)` | The `WorldSnapshot` the view reads |
| `constants` | Every tunable number of the rules, named after its rule id |

## Invariants
- `apply` is transactional: on `Err` the state is unchanged (checked by tests that compare before and after).
- Every owned tile belongs to exactly one territory; a territory with tiles has exactly one capital once R-CAP-02 lands.
- Sea tiles never carry cover, owner or unit.
- Determinism: same state + same command → same events and state on every platform (`cl_noise::Mulberry32` is the only randomness).

## Testing
Unit tests per rule id on `GraphBoard::flower()` and small graphs; a proptest that random legal
command sequences keep the invariants; `insta` snapshots of scripted games once the move rules land.

## Non-goals
Turn orchestration, undo, replays (`cl-session`), AI (`cl-ai`), level parsing (`cl-level`).
