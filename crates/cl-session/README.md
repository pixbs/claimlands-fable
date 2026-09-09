# cl-session

## Purpose
One running match. `Session` owns the level, the hex sphere, the `GameState`, and the command log.
It routes commands from the human or the AI policy into `cl-rules`, keeps the state at the start of
the current turn so any move can be undone until the turn ends (R-TURN-03), and can serialise the
whole match as a `Recording` (level string + commands) that replays deterministically without AI.

## Public API
| Item | Role |
|---|---|
| `Session::start(level)` | New match from a `Level`; capitals from the level get the starting treasury |
| `Session::apply(cmd)` | Applies for the current faction, logs on success, snapshots the turn on `EndTurn` |
| `Session::undo()` | Removes the last command of the current turn by replaying from the turn start |
| `Session::play_ai()` | Lets an AI faction take its whole turn; returns the events |
| `Session::snapshot()` | `WorldSnapshot` for the view |
| `Session::recording()` / `Session::replay(&Recording)` | Save and reproduce a match |
| `PlayerKind` | `Human` or `Ai(Profile)` per faction |

## Invariants
- `undo()` restores exactly the state before the undone command (tested by comparison).
- `replay(recording())` reproduces the current state (tested by comparison).
- The log only contains accepted commands.

## Testing
Unit tests on tiny levels (`seed=0` plus explicit tiles) for undo, replay and AI turns.

## Non-goals
Rendering, input, networking (multiplayer arrives later as another command source).
