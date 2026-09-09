# cl-render

## Purpose
The only crate that talks to the GPU. It reproduces the prototype's presentation pipeline
(`docs/design/visuals.md`): the scene is drawn into an offscreen `Rgba8Unorm` target of
`round(width / pixelScale)` × `round(height / pixelScale)` texels and blitted to the swapchain with
nearest sampling, so one render texel is exactly `pixelScale` screen pixels and the halo, stars and
planet share one pixel grid. Raw prototype colour values are what reaches the screen: when the
swapchain is sRGB the blit converts once so the hardware encode cancels it.

## Public API
| Item | Role | Status |
|---|---|---|
| `Gpu::new(target, width, height, pixel_scale)` | Adapter, device, surface, offscreen target, blit pipeline | ported |
| `Gpu::resize`, `Gpu::set_pixel_scale` | Keep target and surface in step with the window | ported |
| `Gpu::frame()` → `Frame` | Acquire the swapchain image and an encoder; `Frame::low_res_pass(clear)` draws into the target, `Frame::finish()` blits and presents | ported |
| `Frame::surface_view()` | Full-resolution view for the UI layer | ported |
| `SurfaceTarget` re-export | Anything wgpu can make a surface from (winit window, canvas) | ported |
| Lambert / unlit / cloud materials, mesh upload, depth, picking helpers | prototype materials table | issue M1 |

## Invariants
- No `unsafe` outside surface creation (which wgpu wraps).
- Offscreen size follows `cl_noise::js::round`, the prototype's `Math.round(w / pixelScale)`.
- The blit is the only place colour space is touched.

## Testing
The target-size arithmetic is unit tested. GPU behaviour is checked by the web build in a browser
and, from M1, by the headless screenshot regression test.

## Non-goals
Scene content (`cl-scenery`), UI drawing (`cl-ui`), windowing and input (`cl-app`).
