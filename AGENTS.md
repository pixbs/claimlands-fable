# Claim Lands — agent guide

Turn-based territory game on a procedurally generated hex-sphere planet, for iOS, Android and the
web. Rust workspace. The visual prototype `reference/hex-planet.html` is the frozen oracle: the
game must look exactly like it, and `fixtures/` pins the numbers it produces.

## Map

| Path | Owns | Depends on |
|---|---|---|
| `crates/cl-model` | shared types: ids, factions, tile state, snapshots, `MeshData`, `RgbaImage`, `Board` | — |
| `crates/cl-noise` | hashes, value noise, fBm, mulberry32, JavaScript number semantics | libm |
| `crates/cl-hexsphere` | Goldberg sphere: tiles, neighbours, corner frames, facet frames | model, noise |
| `crates/cl-worldgen` | continents and initial cover from a seed | model, hexsphere, noise |
| `crates/cl-pixelart` | CPU textures: strips, ground atlas, cloud sky, halo, palette | model, hexsphere, noise |
| `crates/cl-scenery` | procedural meshes from a `WorldSnapshot` | model, hexsphere, noise, pixelart |
| `crates/cl-rules` | the game as a pure state machine (`apply`), territories, economy, units | model, noise |
| `crates/cl-level` | the one-line level string; starting world | model, hexsphere, worldgen |
| `crates/cl-ai` | AI profiles and policies | model, rules, noise |
| `crates/cl-session` | one match: players, turn flow, undo, replay, stats | model, rules, ai, level, hexsphere |
| `crates/cl-render` | wgpu: pixel-scaled target, blit, materials | model, noise |
| `crates/cl-ui` | egui HUD, menus, debug panel, theme | model |
| `crates/cl-app` | winit shell, input, scene sync, platform entry points (web, Android, iOS) | everything |
| `xtask` | `cargo xtask …`: checks, lints, web bundle, mobile builds, issue picking, scaffolding | — |
| `platforms/` | web page shell, Android Gradle project, iOS project spec | — |
| `reference/` | the frozen prototype and the Node harness that extracts fixtures from it | — |
| `fixtures/` | golden values extracted from the prototype; tests replay them | — |
| `docs/` | architecture, workflow, testing, style, design specs, ADRs | — |

The dependency direction is enforced by `cargo xtask lint-repo` (allow-list in `xtask/src/lint.rs`).
Core crates never depend on wgpu, winit, egui, web-sys or `rand`.

## Commands

| Command | When |
|---|---|
| `cargo xtask setup` | once per clone: targets, cargo tools, git hooks |
| `cargo xtask check` | before every push: fmt, clippy (host + wasm), tests, docs, deny, lint-repo — the same gates as CI |
| `cargo xtask test` / `fmt` / `clippy` / `lint-repo` | the individual gates |
| `cargo xtask web` then `cargo xtask serve` | build the web game into `dist/` and open http://localhost:8080 (prototype at `/reference/`) |
| `cargo xtask fixtures` | regenerate `fixtures/` from the prototype (only after changing the harness) |
| `cargo xtask next` / `cargo xtask take [N]` | list or claim available issues (below) |
| `cargo xtask new-crate cl-x` / `new-adr "title"` | scaffolding with the required sections |
| `cargo xtask android` / `ios` | mobile builds (CI runs them on `main` and nightly) |

## Picking work

1. `cargo xtask next` lists open issues labelled `ready` with no assignee and no open blocker, earliest
   milestone first.
2. `cargo xtask take [N]` assigns you, creates the worktree `../claimlands-wt/N` on branch
   `<type>/N-<slug>`, and prints the issue. Work there; several agents work in parallel this way.
3. Read the issue's acceptance criteria and references before writing code. If the issue needs a
   decision the owner has not made, open a *Design question* issue and stop.
4. One issue → one pull request. Split large work into a stack with `gh stack` (see `docs/workflow.md`).

## Workflow

- Branch: `<type>/<issue>-<slug>` with type `feat|fix|docs|chore|task|refactor|perf|test|ci|build`.
- Commit messages: plain descriptions of the change. No trailers, no co-authors, no tool names.
- PR title: `type(scope): summary` (scope = crate name, e.g. `feat(cl-rules): town feeding`).
  It becomes the squash commit on `main`. PR body starts with `Closes #N`.
- `cargo xtask check` green before opening the PR. CI runs the same gates plus a Cloudflare preview
  of the web build; the preview link appears as a PR comment. Review that link yourself.
- Merge is squash-only after the owner's review. Do not merge your own PR.

## Hard rules

- Never write the name of any AI tool or vendor into commits, branches, PR text, code or docs.
  `.github/policy/banned.txt` lists the patterns; hooks and CI enforce them. Do not bypass hooks.
- Never change `reference/hex-planet.html` or hand-edit `fixtures/`. A visual difference from the
  prototype is a bug unless the issue says otherwise and the PR carries the `visual-change` label.
- Determinism: generation math uses `f64`, `libm` and `cl_noise`; no `f64::sin`-style std calls, no
  `HashMap` iteration, no time or randomness outside `cl_noise::Mulberry32` (`docs/design/porting.md`).
- `TODO` only as `TODO(#issue)`. New dependencies need the `deps` label and a sentence in the PR.
- Public API changes update the crate README and, if a decision changed, an ADR.
- Every rule of the game has an id in `docs/design/rules.md`; its tests carry the id in their names.

## Definition of done

- Acceptance criteria of the issue are met and tested (unit, property, snapshot or fixture).
- `cargo xtask check` passes locally; CI is green; the preview shows the change working.
- Crate README and docs reflect the new state; no dead code, no unexplained constants.
- The PR describes what changed, why, and how it was verified, in the words a reviewer needs.

## Read next

`docs/architecture.md` (how the crates fit), `docs/workflow.md` (issues, stacks, previews, policy),
`docs/testing.md` (fixtures and tolerances), `docs/style.md` (code and prose), `docs/design/*.md`
(rules, level format, visuals, porting, AI), `docs/adr/` (why things are the way they are).
