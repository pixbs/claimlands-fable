# Roadmap

Milestones are GitHub milestones; every item is an issue. `cargo xtask next` serves the earliest
open milestone first.

| Milestone | Outcome | Main issues |
|---|---|---|
| M0 Foundation | workspace, gates, previews, fixtures, walking skeleton on the web | done in the first PRs |
| M1 Planet port | the prototype's planet rendered by the Rust build, pixel for pixel: ground atlas, terrain mesh and cliffs, surf, fields, forests, villages, clouds with see-through, halo, stars, borders, hover, trackball and zoom, picking; fdlibm `sin`/`cos`/`pow`; screenshot regression test | one parent issue per prototype section with sub-issues |
| M2 Rules core | every rule in `docs/design/rules.md` implemented and tested: movement and capture, upkeep and starvation, town feeding, forest growth, capital relocation, split and merge, victory, stats | one parent issue per rule group |
| M3 Playable slice | a match against tier-0/1 AI on the web: HUD with treasury and actions, unit stars, end turn, undo, camera follow, victory screen | app and UI issues |
| M4 Levels and editor | campaign level set, CLI editor, level select screen | level and content issues |
| M5 AI tiers | tiers 1–3 and hostility, campaign difficulty curve | ai issues |
| M6 Victory, replay, polish | statistics, replay viewer, sound hooks, reduced motion | session and UI issues |
| M7 Mobile packaging | signed builds, touch polish, store assets | platform issues |

Later, outside the current plan: multiplayer (another command source into `cl-session`), the
in-game level editor and sharing, a science tree (a new crate consuming rules events).
