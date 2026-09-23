// Weapon skin shader: extends bevy's StandardMaterial with
//   * edge wear (fwidth of the world normal + noise breakup)
//   * optional UV scrolling and emissive pulsing
//   * procedural galaxy finishes: a view-ray window into space
//   * pattern-atlas stamping with a per-copy seed (CSGO-style patterns)
//
// All custom uniforms are packed into one struct at binding 100. Individual
// features are compiled in via shader defs (SKIN_WEAR / SKIN_GALAXY /
// SKIN_PATTERN / SKIN_ANIM) so a driver hang can be bisected from one build.
//
// Follows Bevy's official extended-material example: returns FragmentOutput,
// runs alpha_discard, and handles the deferred pipeline.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::view,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct WeaponSkinExt {
    params: vec4<f32>,      // wear, time, seed, galaxy_on
    anim: vec4<f32>,        // pulse_hz, emissive_strength, scroll_x, scroll_y
    palette_a: vec4<f32>,
    palette_b: vec4<f32>,
    palette_c: vec4<f32>,
    galaxy: vec4<f32>,      // density, speed, stars, parallax
    galaxy_b: vec4<f32>,    // star_density, twinkle, core, layers
    pattern: vec4<f32>,     // offset_x, offset_y, strength, enabled
    gradient: vec4<f32>,    // rgb tint, axis flag (0 = x, 1 = y)
    extra: vec4<f32>,       // x = gradient on, y = pattern fixed offset, z/w unused
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> ext: WeaponSkinExt;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var pattern_atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var pattern_sampler: sampler;

const TAU: f32 = 6.28318530718;

/// Fraction of the motif sheet a single gun shows. Fixed so the stamp size
/// isn't another authoring knob; motifs bake their own feature scale in.
const STAMP: f32 = 0.5;

/// Turns the view ray into a usable window scale: the gun subtends only a
/// fraction of a radian, so the volume needs a multiplier to show structure.
const WINDOW_SCALE: f32 = 6.0;

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// 5-octave value noise, unrolled (no loop).
fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.5 * vnoise(p);
    value += 0.25 * vnoise(p * 2.02);
    value += 0.125 * vnoise(p * 4.0804);
    value += 0.0625 * vnoise(p * 8.242408);
    value += 0.03125 * vnoise(p * 16.649616);
    return value;
}

#ifdef SKIN_GALAXY
// ── Galaxy: a window into space ──────────────────────────────────────────
// Follows the reference Blender recipe: 4D noise (its 4th dimension driven
// by time) through a colour ramp for the nebula, a voronoi-style point field
// for the stars, and the camera's view vector fed into both so the surface
// reads as a window into a fixed volume of space rather than a texture
// painted onto the mesh. Layers sit at increasing depth, so moving the view
// parallaxes them apart.

/// 3D hash → 0..1 (same construction as `hash21`).
fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

/// Value noise in 3D. Time is folded into the domain (the 4th dimension of
/// the reference noise, sampled at w = t).
fn vnoise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let x00 = mix(hash31(i), hash31(i + vec3<f32>(1.0, 0.0, 0.0)), u.x);
    let x10 = mix(hash31(i + vec3<f32>(0.0, 1.0, 0.0)), hash31(i + vec3<f32>(1.0, 1.0, 0.0)), u.x);
    let x01 = mix(hash31(i + vec3<f32>(0.0, 0.0, 1.0)), hash31(i + vec3<f32>(1.0, 0.0, 1.0)), u.x);
    let x11 = mix(hash31(i + vec3<f32>(0.0, 1.0, 1.0)), hash31(i + vec3<f32>(1.0, 1.0, 1.0)), u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

/// Nebula cloud: 4-octave 3D fbm sliced along the view ray, colour-ramped
/// through the skin palette with dark dust lanes rifted through it. The
/// ramp is deliberately dark: only the densest knots light up, everything
/// else stays deep-space black.
fn nebula_color(q: vec3<f32>, t: f32, density: f32) -> vec3<f32> {
    let drift = vec3<f32>(t * 0.35, -t * 0.22, t * 0.15);
    let p = q * density;
    var n = vnoise3(p + drift) * 0.5;
    n += vnoise3(p * 2.03 - drift * 0.7) * 0.25;
    n += vnoise3(p * 4.07 + drift * 0.4) * 0.125;
    n += vnoise3(p * 8.11 - drift * 0.2) * 0.0625;
    // Contrast: fbm averages ~0.476 (measured), so lift the floor above it
    // and expand: most of the surface stays black space and only the dense
    // knots ramp up through the palette.
    let ramp = clamp((n - 0.46) * 2.6, 0.0, 1.0);

    // Colour ramp: deep space → palette A → B → C → a touch of hot white.
    var col = mix(vec3<f32>(0.008, 0.008, 0.020), ext.palette_a.rgb, smoothstep(0.0, 0.45, ramp));
    col = mix(col, ext.palette_b.rgb, smoothstep(0.35, 0.72, ramp));
    col = mix(col, ext.palette_c.rgb, smoothstep(0.66, 0.95, ramp));
    col += vec3<f32>(1.0, 0.97, 0.94) * smoothstep(0.93, 1.0, ramp) * 0.35;

    let dust = vnoise3(p * 3.31 - drift);
    col *= 0.25 + 0.75 * smoothstep(0.20, 0.75, dust);
    return col;
}

/// One layer of stars: jittered points inside 3D cells with a cross flare
/// and a twinkle. `p` is a point along the world-space view ray, so the
/// field is anchored in the world and pans as the view turns.
fn star_layer(p: vec3<f32>, t: f32, amount: f32) -> vec3<f32> {
    let cell = floor(p);
    let f = fract(p) - 0.5;
    let rnd = hash31(cell);
    let o = (vec3<f32>(hash31(cell + 17.0), hash31(cell + 43.0), hash31(cell + 91.0)) - 0.5) * 0.7;
    let rel = f - o;
    let d = length(rel);
    let size = fract(rnd * 534.0);
    // Inverse falloff keeps stars point-like at any resolution.
    let core = 0.016 / max(d, 0.005) * smoothstep(0.93, 0.25, size);
    // Cross flare: lights up along the cell axis nearest the star.
    let flare = exp(-min(min(abs(rel.x), abs(rel.y)), abs(rel.z)) * 55.0) * 0.3;
    let tint = mix(vec3<f32>(0.55, 0.65, 1.0), vec3<f32>(1.0, 0.85, 0.6), fract(rnd * 91.7));
    let twinkle = 1.0 + ext.galaxy_b.y * 0.6 * sin(t * 2.5 + rnd * TAU);
    return tint * (core + flare * core) * twinkle * amount * smoothstep(1.0, 0.35, d);
}

/// The whole effect: a stack of layers at increasing depth, each sampled
/// along the world-space view ray. Turning the camera pans across a fixed
/// volume of space; the per-layer depths add parallax between them.
fn galaxy_color(ray: vec3<f32>, t: f32) -> vec3<f32> {
    let density = max(ext.galaxy.x, 0.05);
    let speed = ext.galaxy.y;
    let star_amount = ext.galaxy.z;
    let parallax = max(ext.galaxy.w, 0.0);
    let star_density = max(ext.galaxy_b.x, 1.0);
    let core = ext.galaxy_b.z;
    let layers = clamp(ext.galaxy_b.w, 1.0, 4.0);

    // Nebula: back-to-front layers; near layers are larger, brighter and
    // drift faster, far ones stay put.
    var col = vec3<f32>(0.0);
    var total = 0.0;
    for (var i = 0u; i < 3u; i++) {
        let fi = f32(i);
        let on = step(fi + 0.5, layers);
        let depth = 1.0 + fi * (1.0 + parallax);
        let q = ray * depth * WINDOW_SCALE + vec3<f32>(fi * 37.0, fi * 19.0, t * speed);
        let weight = on / (1.0 + fi * 1.4);
        col += nebula_color(q, t * speed, density * (0.8 + fi * 0.35)) * weight;
        total += weight;
    }
    col /= max(total, 0.001);

    // Galactic plane + core: a faint band and a hot centre.
    let band = exp(-abs(ray.y) * 6.0) * core;
    let core_dir = normalize(vec3<f32>(0.35, 0.08, -0.93));
    let core_glow = pow(max(dot(ray, core_dir), 0.0), 8.0) * core * 1.6;
    col += ext.palette_b.rgb * band * 0.25 + vec3<f32>(1.0, 0.95, 0.9) * core_glow * 0.2;

    // Stars: nearer layers are sparser and brighter.
    for (var i = 0u; i < 3u; i++) {
        let fi = f32(i);
        let on = step(fi + 0.5, layers);
        let depth = 1.0 + fi * (0.6 + parallax * 0.8);
        let p = ray * depth * star_density + vec3<f32>(fi * 91.0, fi * 57.0, 0.0);
        let amount = on * star_amount / (1.0 + fi * 0.9);
        col += star_layer(p, t, amount);
    }

    return col;
}
#endif

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var vin = in;

#ifdef SKIN_ANIM
    // UV scroll must happen before textures are sampled.
    if (ext.anim.z != 0.0 || ext.anim.w != 0.0) {
        vin.uv = vin.uv + vec2<f32>(ext.anim.z, ext.anim.w) * ext.params.y;
    }
#endif

    var pbr_input = pbr_input_from_standard_material(vin, is_front);
    pbr_input.material.base_color =
        alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var base = pbr_input.material.base_color;

#ifdef SKIN_PATTERN
    // Sample a fixed-size crop of the grayscale motif sheet. The crop origin
    // is either the skin's fixed offset or a hash of the per-copy seed, so
    // duplicates get different pieces of the sheet. Sampled unconditionally
    // (uniform control flow) with an explicit LOD: implicit-derivative
    // sampling is a common driver-hang suspect.
    var crop = ext.pattern.xy;
    if (ext.extra.y < 0.5) {
        crop = vec2<f32>(
            hash21(vec2<f32>(ext.params.z, 1.3)),
            hash21(vec2<f32>(ext.params.z, 7.7)),
        ) * (1.0 - STAMP);
    }
    crop = clamp(crop, vec2<f32>(0.0), vec2<f32>(1.0 - STAMP));
    let stamped = textureSampleLevel(pattern_atlas, pattern_sampler, vin.uv * STAMP + crop, 0.0);
    let pattern_on = step(0.5, ext.pattern.w);
    let strength = clamp(ext.pattern.z, 0.0, 1.0) * pattern_on;
    // Grayscale sheet is centred on mid-grey, so *2 keeps the tint and
    // darkens/brightens with the pattern.
    base = vec4<f32>(mix(base.rgb, base.rgb * stamped.rgb * 2.0, strength), base.a);
#endif

#ifdef SKIN_GRADIENT
    if (ext.extra.x > 0.5) {
        let t = select(vin.uv.x, vin.uv.y, ext.gradient.w > 0.5);
        base = vec4<f32>(mix(base.rgb, ext.gradient.rgb, clamp(t, 0.0, 1.0)), base.a);
    }
#endif

    var emissive = pbr_input.material.emissive.rgb;

#ifdef SKIN_GALAXY
    if (ext.params.w > 0.5) {
        // Camera-dependent sampling: the world-space ray from the eye to
        // this fragment. The surface acts as a window into a volume fixed in
        // the world, so turning the camera pans across it; the per-layer
        // depths add parallax.
        let ray = normalize(pbr_input.world_position.xyz - view.world_position);
        let col = galaxy_color(ray, ext.params.y * 0.35);
        base = vec4<f32>(col, base.a);
        pbr_input.material.metallic = 0.0;
        pbr_input.material.perceptual_roughness = 0.6;
        // Replace the authored emissive with the galaxy's own glow: mostly
        // black space, with the dense knots and stars lighting themselves.
        emissive = col * max(ext.anim.y, 0.15) * 0.45;
    }
#endif

#ifdef SKIN_ANIM
    // Steady or pulsing emissive glow (galaxy handles its own glow above).
    if (ext.params.w <= 0.5 && ext.anim.y > 0.0 && ext.anim.x != 0.0) {
        let pulse = 0.75 + 0.25 * sin(ext.params.y * ext.anim.x * TAU);
        emissive *= pulse;
    }
#endif

#ifdef SKIN_WEAR
    if (ext.params.x > 0.0) {
        // High normal variance across a pixel = a silhouette/sharp edge.
        let edge = smoothstep(0.08, 0.55, length(fwidth(pbr_input.world_normal)));
        let p = pbr_input.world_position.xyz;
        let breakup = fbm(p.xz * 7.0 + p.y * 3.5);
        let scuff = clamp(edge * (0.35 + 0.65 * breakup), 0.0, 1.0) * ext.params.x;

        let scuff_color = vec3<f32>(0.62, 0.62, 0.65);
        base = vec4<f32>(mix(base.rgb, scuff_color, scuff * 0.85), base.a);
        pbr_input.material.perceptual_roughness = mix(
            pbr_input.material.perceptual_roughness, 0.82, scuff);
        pbr_input.material.metallic = mix(pbr_input.material.metallic, 0.95, scuff);
        // Freshly exposed metal loses the glow a little.
        emissive *= 1.0 - scuff * 0.7;
    }
#endif

    pbr_input.material.base_color = base;
    pbr_input.material.emissive = vec4<f32>(emissive, pbr_input.material.emissive.a);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    return out;
}
