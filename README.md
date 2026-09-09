# Claim Lands

A turn-based territory game on a procedurally generated hex-sphere planet: up to four factions
expand, farm, build villages and field pawns, warriors and knights, paying in wheat and gold. Every
texture and model is generated at runtime from a seed, in the pixel-art look of the prototype at
`reference/hex-planet.html`. Targets: iOS, Android and the web.

Play the latest `main`: https://claimlands-fable.pages.dev — every pull request gets its own preview.

## Repository

| Path | Contents |
|---|---|
| `crates/` | Rust workspace: pure core (`cl-model` … `cl-session`), procedural visuals (`cl-pixelart`, `cl-scenery`), renderer (`cl-render`), UI (`cl-ui`), app shell (`cl-app`) |
| `xtask/` | `cargo xtask …`: every check, build and workflow command |
| `platforms/` | web page shell, Android and iOS projects |
| `reference/` | the frozen prototype and the harness that extracts golden fixtures from it |
| `fixtures/` | golden values the crates must reproduce |
| `docs/` | architecture, workflow, testing, style, design specs, decision records |

Start with `AGENTS.md` (the agent guide; humans use the same one) and `docs/architecture.md`.

## Building

```bash
cargo xtask setup      # targets, tools, git hooks
cargo xtask check      # every gate CI runs
cargo xtask web        # web bundle into dist/
cargo xtask serve      # http://localhost:8080
```

There is no desktop build: local runs use the web bundle in a browser.

## Status

Milestone M0 (foundation): workspace, gates, previews, fixtures, walking skeleton. The planet
port (M1), rules (M2), playable slice (M3), levels and editor (M4), AI (M5), victory and replays (M6)
and mobile packaging (M7) are tracked as GitHub issues; `docs/roadmap.md` lists the milestones.

## License

Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International (`LICENSE`): share and adapt
with attribution, non-commercially, under the same terms.
