# Supplemental design references

These executable HTML/JavaScript studies record the approved capital, original unit markers and
availability star. They are inputs to future Rust work, not game integration. The frozen
[`hex-planet.html`](../hex-planet.html) remains the oracle for existing scenery and rendering.

## Files and implementation ownership

Each design has its own page and model source. Keep the sibling files together when copying the pack.

| Design | Interactive page | Model source | Rust issue |
|---|---|---|---|
| Procedural capital | [capital.html](capital.html) | [capital-layout.js](capital-layout.js), [capital.js](capital.js) | [#72](https://github.com/pixbs/claimlands-fable/issues/72) |
| Original Pawn A | [pawn-a.html](pawn-a.html) | [pawn-a.js](pawn-a.js) | [#34](https://github.com/pixbs/claimlands-fable/issues/34) |
| Original Pawn B, hat | [pawn-b.html](pawn-b.html) | [pawn-b.js](pawn-b.js) | [#34](https://github.com/pixbs/claimlands-fable/issues/34) |
| Original Warrior, kite shield | [warrior.html](warrior.html) | [warrior.js](warrior.js) | [#34](https://github.com/pixbs/claimlands-fable/issues/34) |
| Original Knight, pennant | [knight.html](knight.html) | [knight.js](knight.js) | [#34](https://github.com/pixbs/claimlands-fable/issues/34) |
| Availability star | [star.html](star.html) | [star.js](star.js) | [#34](https://github.com/pixbs/claimlands-fable/issues/34) |

The capital decision resolves [#57](https://github.com/pixbs/claimlands-fable/issues/57).
Issue #34 requires `build_units` and `build_stars` returning Rust `MeshData`; it remains open.
Pawn A and Pawn B are alternative appearances for the same pawn kind, not additional unit ranks.
This pack excludes the later compact figurines, doubled figures and fixed castle compositions.

`kit.js` contains shared polygon helpers, the palette and comparison scenery. `viewer.js` contains
orbit, framing and distance controls. The comparison village and forest are scale guides, not
replacements for the frozen village/forest builders or their fixture tests.

## Open and inspect

Open any HTML file in a WebGL browser. The pages also work directly from a local checkout without a
build step. They load the same pinned Three.js r128 CDN as the prototype, so the first load needs
network access. Each page links to the other five designs.

Drag to orbit and tilt; scroll to zoom. The pawn pages switch between a figure close-up and placement
above empty land, forest or village. Their three small views use exactly 1, 2 and 3 screen pixels per
world pixel. The large inspection view renders at half its CSS size with nearest upscaling; zoom
changes its screen density, not model dimensions. Reduced motion stops the automatic star rotation.

PR previews publish these files at `/reference/designs/capital.html`; `/reference/` still serves the
frozen globe. `cargo xtask web` alone copies only the frozen prototype. The preview workflow copies
this supplemental directory after the game build.

## Scale, surfaces and palette

One study unit is one world pixel (WP). The tile texture is 24 × 24 texels, with radius
`24 / 2 * 0.94 = 11.28 WP`. The Rust port converts these local coordinates using the prototype's
`PX = meanR * RADIUS / (TILE_PX / 2 * UV_INSET)` and the tile tangent frame.

Use flat polygon surfaces, nearest texture sampling, no mipmaps, flat Lambert normals, no
antialiasing and the prototype's linear output encoding. Ambient is `#b9c6ff × 0.66`; sun is
`#fff2d8 × 0.52` from `(1.1, 0.9, 1.4)`. Flags and shields are double-sided planes, not voxel stacks.
Small numerical offsets separate decals; they are not new pixel-sized decorative features.

| Surface | Colour values |
|---|---|
| Walls and recessed stone | `#e7ddc8`, `#a99b7e` |
| Roof and roof shadow | `#a24b32`, `#823a26` |
| Wood and openings | `#665034`, `#453227` |
| Grass | `#3f7d34`, `#5aa444`, `#7cc255` |
| Bare ground | `#a18a5b`, `#c9ac72`, `#ddbd7d` |
| Cliff, top to bottom | `#5a4a2e`, `#7d6a45`, `#a08a5f`, `#bda677` |

The prototype supplies the scenery palette. Unit skin `#e2c599`, metal `#bac2ba`, muted gold star
facets and the default blue `#4c92b2` are supplemental accents for objects absent from that prototype.
Castle paths use `#ad956a`, `#b8a277`, `#c4b08a`. These accents are kept from the approved designs;
the alternative image-derived scenery palette is removed. Faction colour is a parameter; changing
it must not change the geometry or castle seed.

## Procedural capital

`CapitalLayout.generate(seedText, family)` produces the complete layout. Family is `A`, `B`, `C` or
`mixed`. `validate(plan)` checks footprint fit, building intersections, clear entrance paths and
keep height. Families describe generation rules, not a selection of stored castle meshes.

| Family | Composition |
|---|---|
| A | 5- or 7-WP keep, 6–8 WP high, optional smaller attached wing |
| B | Two front gate towers, a taller rear keep, optional curtain walls |
| C | Four corner towers, enclosed courtyard, taller inner keep and shifted entrance |

The UTF-16 seed string is hashed with FNV-1a using 32-bit wrapping arithmetic, then sampled with
Mulberry32. Family selection uses `seed + "|family"`; candidate layout uses
`seed + "|" + family + "|" + attempt`. Candidates retry up to 64 times. Preserve evaluation order
when porting. The Rust integration must derive a stable seed string from world seed and tile id;
the study accepts an explicit seed rather than defining that game-level identity contract.

Footprints stay at least 0.5 WP inside the regular hex. Walls, merlons and windows are 1 WP;
the keep door is 2 × 2 WP. B/C gates have a 3-WP opening and at least 2 WP headroom. The clear
entrance path is at least 2 WP wide. The keep is taller than the other buildings. The banner is
a flat 4 × 3 WP pennant on a 1-WP pole, 4 or 5 WP high. The castle rotates in 60-degree increments;
paint sampling follows the same layout orientation. The renderer uses a 0.015-WP separation offset.

`capital.js` builds the approved tower, wall, merlon, opening and banner geometry. The layout is
independent of colour, ground paint, camera and animation. `verify.cjs` checks 1,000 seeds in each
family mode. Rust implementation and scene refresh belong to #72 and #20, not this reference pack.

## Original units

Model builders return a group at local origin, facing +Z. The shared beveled base is
5.6 × 1 × 4.2 WP; the tunic tapers from 4.2 × 3.5 to 2.8 × 2.8 WP across 3.2 WP of height.
These approved fractional silhouette dimensions are retained. They do not introduce a finer
texture grid. Box helper Y coordinates specify the bottom, not the centre.

| Model | Full height | Distinguishing construction |
|---|---|---|
| Pawn A | 7 WP | Skin stops at Y=6; hair begins there, with a rear hair plane offset 0.02 WP |
| Pawn B | 8.4 WP | Flat 5.6-WP brim and sloping crown |
| Warrior | 7 WP | Flat kite shield rotated inward 20°, inset band offset 0.06 WP |
| Knight | 9.8 WP | Closed helm, narrow visor, pole and flat pennant |

For comparison placement, `unitBottom = ceil(coverTop / WP) * WP + 2 * WP`. Preserve this clearance
above forest or village rather than scaling down the marker to fit the scenery. Pawn selection
and game availability come from game state, not the viewer controls.

## Availability star

The five-point outline has 15 vertices: two clipped-tip corners and one inner corner per point.
Nominal radius is 3 WP; inner radius is 1.55 WP; tip clipping is 0.45 WP. The edge is 1 WP deep
and the two centre facets protrude to Z=±1 WP. `star.js` is the shared mesh and animation source.

Place its centre at `ceil(subjectTop / WP) * WP + 2 * WP + 3 * WP`, where subject top includes
the unit pennant or castle pole. Apply a camera-oriented mount followed by a local Y rotation:
`cameraQuaternion * rotationY(seconds * 2π / 8)`. One revolution takes 8 seconds. It intentionally
turns edge-on at quarter revolutions; the mount follows the camera, but the rotating face is not
a permanently front-facing billboard. Animation time must not consume layout randomness.

R-UI-01 controls stars over units that can act; R-UI-02 controls stars over capitals that can afford
an action. The study checkbox only previews visibility. In the game, derive it from the snapshot.
The viewer pauses automatic motion when hidden or reduced motion is requested.

## Verification

Run `node reference/designs/verify.cjs` for deterministic generation, footprint/path checks,
composition diversity, JavaScript syntax and local file links. The preview workflow runs this
check before publishing. Browser verification of this extraction compared all four model meshes,
normals, transforms and colours with their approved originals; checked their heights and the star's
Y rotation at 0, 2 and 8 seconds; and exercised all six pages at desktop and mobile sizes.

Run `cargo xtask check` for the repository gates. Existing Rust, the frozen prototype and its
fixtures are unchanged by this pack.
