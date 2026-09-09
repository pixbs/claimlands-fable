# Visuals

The prototype `reference/hex-planet.html` defines the look. This page lists what the renderer must
reproduce and where each part lives. Numbers live in `fixtures/constants.json` (every top-level
constant of the prototype) and in the crate that ports them; tests compare the two.

## Presentation pipeline

| Step | Prototype | Port |
|---|---|---|
| Render size | `round(w / pixelScale)` × `round(h / pixelScale)` CSS pixels, `setPixelRatio(1)`, `antialias: false`, canvas upscaled with `image-rendering: pixelated` | `cl-render`: `Rgba8Unorm` target of that size, blitted to the swapchain with nearest sampling |
| Colour space | `LinearEncoding`, no tone mapping: hex values reach the screen unchanged | blit converts sRGB→linear once when the swapchain is sRGB so the hardware encode restores raw values |
| Pass 1 | orthographic space pass: vignette plane (15×15 vertex colours) and pixel-snapped star quads, depth off | `cl-scenery::build_space` (M1), unlit pipeline |
| Depth clear | `renderer.clearDepth()` between passes | separate render pass |
| Pass 2 | perspective camera fov 38°, near 0.1, far 50, on +z at distance 1.35–6 (start 3.3), planet rotates | `cl-app` camera, `cl-render` pipelines |
| Lights | ambient `#b9c6ff` × 0.66; directional `#fff2d8` × 0.52 from `normalize(1.1, 0.9, 1.4)` in world space | Lambert material uniforms |
| Lambert | `out = diffuse × (ambient + saturate(dot(n, l)) × sun)`; `diffuse = material colour × vertex colour × texel`; flat normals per triangle | WGSL in `cl-render` (M1) |

## Materials

| Mesh | Material | Details |
|---|---|---|
| ground | Lambert, atlas map, vertex colours | nearest, no mipmaps; owner tint ×(1.30, 1.00, 0.52) |
| cliff walls | Lambert, cliff strip repeating | `RepeatWrapping` both axes |
| surf | Lambert, foam sheet, `alphaTest 0.5`, double-sided, render order 1 | 8 frames, offset stepped every 140 ms |
| edges (debug) | unlit `#1b2c22`, double-sided | ribbons inset half a world pixel |
| fields | Lambert, field strip, vertex colour (shade 0.94–1.06) | `RepeatWrapping` u, clamp v |
| posts, forest, houses | Lambert, vertex colours | flat shading, per-facet jitter |
| clouds ×3 | Lambert, deck tint, Bayer alpha, `alphaTest 0.03`, double-sided, render order k | see-through hole in the fragment shader |
| atmosphere | unlit `#c3e3f6`, reversed winding | rim only |
| halo | unlit radial gradient, transparent, no depth write, render order −1, linear filter | quad behind the planet scaled to the atmosphere |
| territory border | unlit `#f2b45c`, double-sided, polygon offset −2/−2, render order 1 | ribbons inset one texel, lifted per cover |
| hover ring | line loop `#58c2b0` | above the tile's cover |

## Clocks and interaction

| Thing | Value |
|---|---|
| Surf animation | 140 ms per frame, 8 frames |
| Star flicker | stepped every 110 ms, levels 1.0 / 0.82 / 0.58 / 0.82, 38 % of stars |
| Cloud drift | 0.0016 turns per second, whole texels |
| See-through | opens from distance 6 to 5, hole 24°→64°, core 40 %, `opacity = 1 − 0.98 × t` |
| Drag | trackball in camera space, `0.0055` rad per pixel, inertia ×0.94 per frame |
| Zoom | wheel ×1.09 per notch, pinch ratio, clamped to 1.35–6 |
| Tap | under 6 px of movement picks the tile under the pointer (cliffs resolve to the tile above) |
| Idle drift (optional) | 0.0009 rad per frame |
| Reduced motion | drift, flicker and surf stop; see-through still follows zoom |

## Fidelity checks

- Fixtures: every texture as PNG, every mesh as exact `f32` hashes (`docs/testing.md`).
- Preview: each PR deploys the game with the prototype at `/reference/` for A/B at the same seed.
- Visual regression (M1): a headless screenshot at a fixed seed and camera compared with a baseline.
