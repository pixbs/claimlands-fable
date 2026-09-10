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
| `Planet` | The ported builders assembled once and drawn every frame, the cloud decks included | ported |
| `Backdrop` | The space pass: vignette and stars, rebuilt when the target size changes, drawn before the depth clear | ported |
| `Camera`, `Trackball` | fov 38°, distance 1.35–6 from 3.3; trackball spin, flick inertia, wheel and pinch | ported |
| `start()` (wasm) | Installs panic and log hooks, appends the canvas, spawns the event loop | ported |
| `android_main` | `GameActivity` entry via `android-activity` | compiled by `mobile.yml` |
| `claimlands_main` | iOS entry called from the Xcode project's `main.m` | compiled by `mobile.yml` |
| Tap picking and the hover ring | prototype section 6 | issue M1 |
| Surf clock | section 8 | ported |
| Cloud decks: one shell, one material per deck; `Planet::set_camera_distance` opens the see-through hole | section 5 (`buildClouds`), the see-through half of `stepSky` | ported |
| Scene sync from `cl-session` events; star and cloud-drift clocks | sections 5, 8 | issues M1, M3 |

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
- The cloud decks share one uploaded shell; their scale rides the model matrix, which the shader
  already takes to be a rotation and a uniform scale. They draw last, being the outermost shells,
  and the depth buffer is what separates them — the order only settles ties between decks.
- The see-through hole tracks the camera's distance, not a clock, so it is set every frame beside
  the model rather than on the animation clocks. The camera stays on `+z`, so the opening always
  faces the viewer and only its width and depth change.
- A frame is two passes into one target: the backdrop clears the colour and writes no depth, then
  the depth clear starts the planet, so nothing behind the planet can ever occlude it.

## Testing
The camera and trackball are unit tested: the zoom range and its wheel and pinch ratios, the basis
`spinBy` reads out of the camera, that the planet stays inside the depth range across the whole zoom
range, and that a flick decays to a stop without the orientation drifting off unit length. The rest
is reviewed on the preview against `/reference/`. Compiles for `wasm32-unknown-unknown`, `aarch64-linux-android` and `aarch64-apple-ios` in CI; behaviour is
reviewed on the web preview and, from M1, by the screenshot regression test.

## Non-goals
Rules, generation, meshes, textures, GPU pipelines beyond wiring them together.
