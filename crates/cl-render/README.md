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
| `Frame::scene_pass(clear_color, clear_depth)` | A pass on the target and its depth buffer; the prototype draws space, clears depth, then the planet | ported |
| `Renderer::new(device, queue, format)` | Shader, layouts, scene uniform and the pipeline cache | ported |
| `Renderer::upload_mesh`, `Renderer::upload_texture` | `MeshData` and `cl_model::Texture` onto the GPU, wrap and filter as the prototype sets them | ported |
| `Renderer::material(desc, uniform, map)`, `Renderer::draw` | One material, one draw | ported |
| `MaterialDesc` presets: `lambert`, `lambert_double_sided`, `cloud`, `unlit`, `unlit_double_sided`, `border`, `halo`, `space` | the rows of the materials table | ported |
| Cloud sky texture and the halo image | `cl-pixelart`, issue M1 | issue M1 |
| Picking helpers | raycast to tile | issue M1 |

## Pipelines
One shader, `src/scene.wgsl`, with two fragment entry points. Everything that varies per mesh is a
uniform, so the pipeline set is one per distinct combination of cull, blend, depth, bias and
topology — six for the whole prototype.

| Stage | Formula |
|---|---|
| vertex | `clip = view_proj × model × position`; `uv = uv × repeat + offset`; the normal takes `model` unchanged, being rotation and uniform scale only |
| diffuse | `material colour × vertex colour × texel`, each optional through a flag |
| hole | `alpha ×= mix(1, open, smoothstep(cos θ_out, cos θ_in, dot(normalize(world), focus)))` |
| discard | `alpha < alphaTest` |
| `fs_lambert` | `diffuse × (ambient + saturate(dot(n, l)) × sun)`, flat normals |
| `fs_unlit` | the diffuse colour alone |

Lights are the prototype's: ambient `#b9c6ff` × 0.66, directional `#fff2d8` × 0.52 from
`normalize(1.1, 0.9, 1.4)`, both multiplied into the uniform on the CPU. No colour-space conversion
happens here — that is the blit's one job.

| Uniform | Bytes | Members |
|---|---|---|
| `SceneUniform` (group 0) | 112 | `view_proj`, `ambient`, `sun_color`, `sun_dir` |
| `DrawUniform` (group 1) | 144 | `model`, `color`, `uv_transform`, `params` (alphaTest, hole outer, hole inner, open), `focus`, `flags` |

`DrawUniform` ends in three scalar pads rather than a `vec3<u32>`, which WGSL would align to 16 and
stretch the block to 160 bytes. Group 1 also carries the map and its sampler; a material without a
map binds one white texel, so the layout never changes.

## Invariants
- No `unsafe` outside surface creation (which wgpu wraps).
- Offscreen size follows `cl_noise::js::round`, the prototype's `Math.round(w / pixelScale)`.
- The blit is the only place colour space is touched.
- Depth lives on the low-resolution target as `Depth24Plus`, cleared between the two passes exactly
  as the prototype calls `renderer.clearDepth()`.
- Opaque materials write RGB only, so an alphaTest discard cannot punch a hole in the target alpha
  the blit reads.

## Testing
The target-size arithmetic, the uniform layouts, the light constants and the material presets are
unit tested. `tests/headless.rs` renders one frame without a window and reads the texels back,
checking them against the Lambert formula computed on the CPU; it needs an adapter, so it prints why
and passes where there is none, and CI installs lavapipe to make it run.

## Non-goals
Scene content (`cl-scenery`), UI drawing (`cl-ui`), windowing and input (`cl-app`).
