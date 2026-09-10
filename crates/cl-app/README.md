# cl-app

## Purpose
The application shell that every platform boots into: a winit event loop, the `cl-render` GPU
context, the egui layer, and (from M1) the planet scene, camera, input and animation clocks. It is a
library with three entry points (`start` for the web, `android_main`, `claimlands_main` for iOS) and
no desktop binary. Local runs use the web build.

## Public API
| Item | Role | Status |
|---|---|---|
| `App` (`winit::application::ApplicationHandler`) | Window, GPU init (async on the web), resize, redraw, input, egui events | ported |
| `Planet` | The ported builders assembled once and drawn every frame | ported |
| `Camera`, `Trackball` | fov 38°, distance 1.35–6 from 3.3; trackball spin, flick inertia, wheel and pinch | ported |
| `start()` (wasm) | Installs panic and log hooks, appends the canvas, spawns the event loop | ported |
| `android_main` | `GameActivity` entry via `android-activity` | compiled by `mobile.yml` |
| `claimlands_main` | iOS entry called from the Xcode project's `main.m` | compiled by `mobile.yml` |
| Tap picking and the hover ring | prototype section 6 | issue M1 |
| Surf clock | section 8 | ported |
| Scene sync from `cl-session` events; star, cloud and see-through clocks | sections 5, 8 | issues M1, M3 |

## Invariants
- The frame is drawn only on `RedrawRequested`; nothing blocks the event loop on the web.
- Physical → logical → target size follows `cl_render::target_size` with `setPixelRatio(1)` semantics:
  one CSS pixel is one logical pixel before the pixel scale.
- Platform code is confined to `platform.rs`, and the only clock in the game is `time::now_ms`:
  `performance.now()` on the web, where `std::time::Instant` panics.
- Dragging spins the planet, never the camera, which stays on `+z` looking at the origin. That is
  what makes the gesture read as a trackball, and it keeps the lights fixed in world space.
- The redraw loop is continuous, as the prototype's `requestAnimationFrame` is: the surf steps and a
  flick decays on a clock.

## Testing
The camera and trackball are unit tested: the zoom range and its wheel and pinch ratios, the basis
`spinBy` reads out of the camera, that the planet stays inside the depth range across the whole zoom
range, and that a flick decays to a stop without the orientation drifting off unit length. The rest
is reviewed on the preview against `/reference/`. Compiles for `wasm32-unknown-unknown`, `aarch64-linux-android` and `aarch64-apple-ios` in CI; behaviour is
reviewed on the web preview and, from M1, by the screenshot regression test.

## Non-goals
Rules, generation, meshes, textures, GPU pipelines beyond wiring them together.
