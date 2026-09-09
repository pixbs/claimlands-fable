# cl-ui

## Purpose
Everything drawn with egui: the in-game HUD (resources, turn, actions), menus and level select, the
victory screen, and the debug panel that mirrors the prototype's instrumentation. The theme copies
the prototype's CSS palette (`#0e0b16` deep, `rgba(27,22,38,.82)` panels, `#2f2740` lines,
`#ece4d4` text, `#8b809c` muted, `#f2b45c` gold, `#58c2b0` teal, monospace type).

## Public API
| Item | Role | Status |
|---|---|---|
| `theme::apply(&egui::Context)` | Installs the palette and monospace defaults | ported |
| `Hud` + `HudInfo` | Top-left readout and the debug toggle; `Hud::ui(ctx, &info) -> HudAction` | skeleton |
| `HudAction` | What the app should do in response (pixel-scale change, debug toggle) | skeleton |
| Game HUD (treasury, turn, build/recruit buttons, end turn, undo), menus, victory | game screens | issues M3, M6 |

## Invariants
- Pure presentation: no game logic, no rendering resources; the app supplies data and applies actions.
- Every interactive control returns its effect through `HudAction`; nothing mutates shared state.

## Testing
Layout logic that computes strings and actions is unit tested; visual review happens on the web preview.

## Non-goals
World visuals, input on the planet, wgpu integration (`cl-app` owns `egui-wgpu`/`egui-winit`).
