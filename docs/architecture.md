# Architecture

## Shape

Ports and adapters. The game is a pure state machine; the visuals are a pure function of a world
snapshot; the app wires them together through events. Nothing in the core knows about the GPU, the
window, time or the platform, so every rule and every mesh builder is tested on the CPU in
milliseconds, and the same code runs unchanged on iOS, Android and the web.

```
                 cl-app  (winit shell: input, scene sync, animation clocks, platform entry points)
                /   |    \                       \
         cl-render  cl-ui  cl-scenery ─── cl-pixelart       cl-session ─── cl-ai
          (wgpu)   (egui)      \             /                  |   \        |
                                cl-hexsphere ── cl-worldgen     cl-level   cl-rules
                                     \              /               \        /
                                      cl-noise ── cl-model ───────────────
```

Arrows point downward: a crate may use what is below it, never what is above. `cargo xtask lint-repo`
checks the allow-list in `xtask/src/lint.rs` on every PR.

## Crates

| Crate | Responsibility | May depend on |
|---|---|---|
| `cl-model` | vocabulary: `TileId`, `Faction`, `Terrain`, `Cover`, `UnitKind`, `TileState`, `WorldSnapshot`, `MeshData`, `RgbaImage`, `Board`, world scale constants | serde |
| `cl-noise` | `hash2`, `hash3i`, `vnoise3`, `fbm3`, `Mulberry32`, V8-exact `hypot`, `to_int32`, `round`, `to_fixed6`, 3-vector kit | libm |
| `cl-hexsphere` | geodesic icosahedron, dual tiles with corners/neighbours/frames, facet planes, texel inverse | model, noise |
| `cl-worldgen` | continents, initial cover clumps | model, hexsphere, noise |
| `cl-pixelart` | strips, ground atlas, cloud sky, halo, palette | model, hexsphere, noise |
| `cl-scenery` | terrain, cliffs, surf, fields, forests, villages, clouds, atmosphere, borders, space pass, picking | model, hexsphere, noise, pixelart |
| `cl-rules` | `GameState`, `Command`, `Event`, `apply`, territories, economy, units, victory, stats | model, noise |
| `cl-level` | level string parse/serialise, starting `WorldSnapshot` | model, hexsphere, worldgen |
| `cl-ai` | `Profile`, `Policy` tiers | model, rules, noise |
| `cl-session` | players, turn flow, undo within a turn, command log, replay | model, rules, ai, level, hexsphere |
| `cl-render` | surface, pixel-scaled target and blit, materials, mesh/texture upload | model, noise, wgpu |
| `cl-ui` | HUD, menus, debug panel, theme | model, egui |
| `cl-app` | window and events, GPU start-up, input, scene sync, animation, entry points | all of the above |

## Data flow

1. `cl-level` turns a level string into a `WorldSnapshot`; `cl-session` builds the `GameState`.
2. A command (human input or `cl-ai`) goes through `cl-session::apply` → `cl-rules::apply`, which
   validates, mutates and returns `Event`s.
3. `cl-app` maps events to dirty regions: tile repaint (ground atlas), cover zone rebuild, border
   rebuild, camera focus. It asks `cl-scenery` for new `MeshData` and `cl-pixelart` for new texels
   only for what changed, mirroring the prototype's `refresh`, `repaint`, `refreshCover`,
   `rebuildBorder`.
4. `cl-render` uploads meshes and textures, draws the space pass and the planet into the
   low-resolution target, blits it to the swapchain, and `cl-ui` draws the HUD on top.

## Isolation rules

- One feature lives in one crate or one module file; the workspace uses `members = ["crates/*"]`,
  so adding a crate touches no shared file except `Cargo.lock`.
- `cl-scenery` reads `WorldSnapshot`, never `GameState`: visuals can be developed with generated or
  hand-made worlds without rules.
- `cl-rules` is testable on any `Board`: the seven-tile `GraphBoard::flower()` exercises every rule.
- Determinism is a property of the core: same inputs give the same bits on every platform
  (`docs/design/porting.md`).

## Decisions

Each decision has an ADR in `docs/adr/`: stack (0001), layering (0002), deterministic math (0003),
fidelity via fixtures (0004), trunk-based stacked PRs (0005), attribution firewall (0006), level
string (0007), previews (0008), crate-per-feature (0009), issues as the work queue (0010).
