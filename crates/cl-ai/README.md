# cl-ai

## Purpose
Computer opponents. A `Profile` (hostility 0–100, intelligence tier 0–3) selects a `Policy`; a policy
reads the `GameState` and `Board` and returns the next `Command`. Policies only ever return commands
`cl-rules` accepts, and they are deterministic for a given seed, so a replay of AI moves needs no AI.

## Public API
| Item | Role |
|---|---|
| `Profile` | Level-defined difficulty knobs |
| `Policy` | `choose(&mut self, &GameState, &dyn Board) -> Command`; returns `EndTurn` when nothing else is wanted |
| `policy_for(profile, seed)` | The policy implementing a tier |
| `Passive` | Tier 0 placeholder: always ends the turn |

## Invariants
- Every returned command is legal for the current faction (tests apply it and assert `Ok`).
- Same state, profile and seed → same command on every platform; randomness is `cl_noise::Mulberry32`.
- No policy inspects anything the rules would not let it act on.

## Testing
Legality of every command a policy emits over scripted states; determinism across two runs.
Tier behaviour tests (expansion rate, aggression) arrive with each tier (issues M5).

## Non-goals
Turn orchestration (`cl-session`), rule evaluation (`cl-rules`), difficulty curves of the campaign (level data).
