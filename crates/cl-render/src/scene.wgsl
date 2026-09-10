// The prototype's two materials, as one shader with two fragment entry points.
//
// Everything is computed in the prototype's colour space: `LinearEncoding` with no tone mapping, so
// a hex value reaches the low-resolution target unchanged. The blit does the one sRGB conversion,
// if the swapchain needs it.

struct Scene {
    view_proj: mat4x4<f32>,
    // rgb premultiplied by the light's intensity; w unused.
    ambient: vec4<f32>,
    sun_color: vec4<f32>,
    // Unit direction toward the sun in world space; w unused.
    sun_dir: vec4<f32>,
};

struct Draw {
    model: mat4x4<f32>,
    // Material colour, w = opacity.
    color: vec4<f32>,
    // three.js `texture.repeat` in xy and `texture.offset` in zw.
    uv_transform: vec4<f32>,
    // x: alphaTest, y: cos of the see-through hole's outer angle, z: its inner angle, w: how far
    // the hole is open (1 = shut).
    params: vec4<f32>,
    // Direction the see-through hole opens toward; w unused.
    focus: vec4<f32>,
    // Three scalar pads, not a vec3: a vec3<u32> would align to 16 and stretch the block to 160
    // bytes, where the Rust struct is 144.
    flags: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
};

const FLAG_TEXTURED: u32 = 1u;
const FLAG_VERTEX_COLOR: u32 = 2u;
const FLAG_CLOUD_HOLE: u32 = 4u;

@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var<uniform> draw: Draw;
@group(1) @binding(1) var map: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    let world = draw.model * vec4<f32>(position, 1.0);
    out.clip = scene.view_proj * world;
    out.world = world.xyz;
    // Rotation and uniform scale only, so the inverse transpose is the matrix itself once the
    // fragment stage normalises.
    out.normal = (draw.model * vec4<f32>(normal, 0.0)).xyz;
    out.uv = uv * draw.uv_transform.xy + draw.uv_transform.zw;
    out.color = color;
    return out;
}

/// `diffuse = material colour × vertex colour × texel`, and the alpha the alphaTest discard reads.
fn surface(in: VsOut) -> vec4<f32> {
    var texel = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    if ((draw.flags & FLAG_TEXTURED) != 0u) {
        texel = textureSample(map, samp, in.uv);
    }
    var rgb = draw.color.rgb * texel.rgb;
    if ((draw.flags & FLAG_VERTEX_COLOR) != 0u) {
        rgb = rgb * in.color;
    }
    var alpha = draw.color.a * texel.a;
    if ((draw.flags & FLAG_CLOUD_HOLE) != 0u) {
        // The hole is decided per fragment: a uniform opacity could only thin the whole deck at
        // once. Lowering alpha inside the cap knocks out low dither ranks at the discard below.
        let cap = dot(normalize(in.world), draw.focus.xyz);
        let hole = smoothstep(draw.params.y, draw.params.z, cap);
        alpha = alpha * mix(1.0, draw.params.w, hole);
    }
    return vec4<f32>(rgb, alpha);
}

@fragment
fn fs_lambert(in: VsOut) -> @location(0) vec4<f32> {
    let s = surface(in);
    if (s.a < draw.params.x) {
        discard;
    }
    // out = diffuse × (ambient + saturate(dot(n, l)) × sun). Normals arrive flat, one per triangle.
    let n = normalize(in.normal);
    let dot_nl = clamp(dot(n, scene.sun_dir.xyz), 0.0, 1.0);
    return vec4<f32>(s.rgb * (scene.ambient.rgb + dot_nl * scene.sun_color.rgb), s.a);
}

@fragment
fn fs_unlit(in: VsOut) -> @location(0) vec4<f32> {
    let s = surface(in);
    if (s.a < draw.params.x) {
        discard;
    }
    return s;
}
