# cl-app

## Purpose
The application shell that every platform boots into: a winit event loop, the `cl-render` GPU
context, the egui layer, and (from M1) the planet scene, camera, input and animation clocks. It is a
library with three entry points (`start` for the web, `android_main`, `claimlands_main` for iOS) and
no desktop binary. Local runs use the web build.

## Public API
| Item | Role | Status |
|---|---|---|
| `App` (`winit::application::ApplicationHandler`) | Window, GPU init (async on the web), resize, redraw, egui events | walking skeleton |
| `start()` (wasm) | Installs panic and log hooks, appends the canvas, spawns the event loop | ported |
| `android_main` | `GameActivity` entry via `android-activity` | compiled by `mobile.yml` |
| `claimlands_main` | iOS entry called from the Xcode project's `main.m` | compiled by `mobile.yml` |
| Trackball drag, pinch and wheel zoom, tap picking, hover ring | prototype section 6 | issue M1 |
| Scene sync from `cl-session` events; surf, star, cloud and see-through clocks | sections 5, 8 | issues M1, M3 |

## Invariants
- The frame is drawn only on `RedrawRequested`; nothing blocks the event loop on the web.
- Physical → logical → target size follows `cl_render::target_size` with `setPixelRatio(1)` semantics:
  one CSS pixel is one logical pixel before the pixel scale.
- Platform code is confined to `platform.rs`; the rest is target-agnostic.

## Testing
Compiles for `wasm32-unknown-unknown`, `aarch64-linux-android` and `aarch64-apple-ios` in CI; behaviour is
reviewed on the web preview and, from M1, by the screenshot regression test.

## Non-goals
Rules, generation, meshes, textures, GPU pipelines beyond wiring them together.
