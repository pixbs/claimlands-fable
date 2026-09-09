# AI opponents

`crates/cl-ai`. A level assigns each AI faction a `Profile { hostility 0–100, intelligence 0–3 }`.
`policy_for(profile, seed)` returns the tier's `Policy`; `Session::play_ai` runs it until it returns
`EndTurn`. Policies are deterministic for a seed, so replays never need the AI.

## Tiers

| Tier | Name | Behaviour | Status |
|---|---|---|---|
| 0 | Passive | ends the turn | done (placeholder for every tier) |
| 1 | Random legal | enumerates legal commands (`cl-rules` helper, M5), picks uniformly, ends the turn when none remain | open |
| 2 | Greedy | scores each legal command by immediate value: income delta, tiles claimed, units threatened, capital safety; hostility weights capture and attack terms | open |
| 3 | Lookahead | greedy with one-ply search over the opponent's best reply and a small opening book for expansion | open |

## Hostility

Scales the weight of aggressive terms (entering enemy tiles, taking units, threatening capitals)
against expansion and economy terms. 0 never attacks a unit or capital; 100 always prefers the
attack when it is legal.

## Campaign curve (M4/M5)

Levels raise intelligence before hostility: a smarter but calm opponent teaches the economy; a
hostile one tests it. The first levels use tier 1 at hostility 20–40.
