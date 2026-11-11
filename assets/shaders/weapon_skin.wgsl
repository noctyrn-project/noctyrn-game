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
    tint: vec4<f32>,        // per-copy pattern colourway (rgb)
    extra: vec4<f32>,       // x = gradient on, y = pattern fixed offset, z = tint on, w unused
    nebula_b: vec4<f32>,    // detail, roughness, lacunarity, distortion
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
//
// Portal modes (`params.w`): 1 = galaxy, 2 = nebula (transcript-faithful),
// 3 = matrix rain. Nebula/matrix reuse the same uniforms with their own
// interpretation — see `to_extension`.

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
/// through the skin palette with soft dust lanes. The ramp is deliberately
/// soft: most of the surface stays deep space and the knots bloom gently
/// instead of clipping to hot white.
fn nebula_color(q: vec3<f32>, t: f32, density: f32) -> vec3<f32> {
    let drift = vec3<f32>(t * 0.35, -t * 0.22, t * 0.15);
    let p = q * density;
    var n = vnoise3(p + drift) * 0.5;
    n += vnoise3(p * 2.03 - drift * 0.7) * 0.25;
    n += vnoise3(p * 4.07 + drift * 0.4) * 0.125;
    n += vnoise3(p * 8.11 - drift * 0.2) * 0.0625;
    // Soft contrast: fbm averages ~0.476, so lift just above it and expand
    // gently — dense knots bloom through the palette without harsh clipping.
    let ramp = clamp((n - 0.44) * 1.9, 0.0, 1.0);

    // Colour ramp: deep space → palette A → B → C → a breath of warm white.
    var col = mix(vec3<f32>(0.012, 0.012, 0.028), ext.palette_a.rgb, smoothstep(0.0, 0.55, ramp));
    col = mix(col, ext.palette_b.rgb, smoothstep(0.35, 0.80, ramp));
    col = mix(col, ext.palette_c.rgb, smoothstep(0.65, 0.97, ramp));
    col += vec3<f32>(1.0, 0.97, 0.94) * smoothstep(0.94, 1.0, ramp) * 0.18;

    let dust = vnoise3(p * 3.31 - drift);
    col *= 0.45 + 0.55 * smoothstep(0.30, 0.85, dust);
    return col;
}

/// One layer of stars: jittered points inside 3D cells with a soft glow
/// and a gentle twinkle. `p` is a point along the world-space view ray, so
/// the field is anchored in the world and pans as the view turns.
fn star_layer(p: vec3<f32>, t: f32, amount: f32) -> vec3<f32> {
    let cell = floor(p);
    let f = fract(p) - 0.5;
    let rnd = hash31(cell);
    let o = (vec3<f32>(hash31(cell + 17.0), hash31(cell + 43.0), hash31(cell + 91.0)) - 0.5) * 0.7;
    let rel = f - o;
    let d = length(rel);
    let size = fract(rnd * 534.0);
    // Soft falloff so stars glow instead of clipping to hard points.
    let core = 0.010 / max(d, 0.012) * smoothstep(0.95, 0.30, size);
    // Wide soft halo: lights up along the cell axis nearest the star.
    let flare = exp(-min(min(abs(rel.x), abs(rel.y)), abs(rel.z)) * 32.0) * 0.18;
    let tint = mix(vec3<f32>(0.65, 0.72, 1.0), vec3<f32>(1.0, 0.88, 0.70), fract(rnd * 91.7));
    let twinkle = 1.0 + ext.galaxy_b.y * 0.35 * sin(t * 1.6 + rnd * TAU);
    return tint * (core + flare * core) * twinkle * amount * smoothstep(1.0, 0.35, d);
}

/// The whole effect: a stack of layers at increasing depth, each sampled
/// along the world-space view ray. Turning the camera pans across a fixed
/// volume of space; the per-layer depths add parallax between them. A slow
/// idle drift keeps the field breathing Fortnite-style even when the skin's
/// own scroll speed is zero.
fn galaxy_color(ray: vec3<f32>, t: f32) -> vec3<f32> {
    let density = max(ext.galaxy.x, 0.05);
    let speed = ext.galaxy.y;
    let star_amount = ext.galaxy.z;
    let parallax = max(ext.galaxy.w, 0.0);
    let star_density = max(ext.galaxy_b.x, 1.0);
    let core = ext.galaxy_b.z;
    let layers = clamp(ext.galaxy_b.w, 1.0, 4.0);

    // Idle drift: slow orbital sway so the galaxy shimmers even at rest.
    let idle = vec3<f32>(sin(t * 0.45) * 0.06, cos(t * 0.38) * 0.06, sin(t * 0.30) * 0.04);

    // Nebula: back-to-front layers; near layers are larger, brighter and
    // drift faster, far ones stay put.
    var col = vec3<f32>(0.0);
    var total = 0.0;
    for (var i = 0u; i < 3u; i++) {
        let fi = f32(i);
        let on = step(fi + 0.5, layers);
        let depth = 1.0 + fi * (1.0 + parallax);
        let q = ray * depth * WINDOW_SCALE + vec3<f32>(fi * 37.0, fi * 19.0, t * speed) + idle * (1.0 + fi * 0.5);
        let weight = on / (1.0 + fi * 1.4);
        col += nebula_color(q, t * speed, density * (0.8 + fi * 0.35)) * weight;
        total += weight;
    }
    col /= max(total, 0.001);

    // Galactic plane + core: a faint band and a soft centre glow.
    let band = exp(-abs(ray.y) * 6.0) * core;
    let core_dir = normalize(vec3<f32>(0.35, 0.08, -0.93));
    let core_glow = pow(max(dot(ray, core_dir), 0.0), 8.0) * core * 1.6;
    col += ext.palette_b.rgb * band * 0.18 + vec3<f32>(1.0, 0.95, 0.9) * core_glow * 0.12;

    // Stars: nearer layers are sparser and brighter.
    for (var i = 0u; i < 3u; i++) {
        let fi = f32(i);
        let on = step(fi + 0.5, layers);
        let depth = 1.0 + fi * (0.6 + parallax * 0.8);
        let p = ray * depth * star_density + vec3<f32>(fi * 91.0, fi * 57.0, 0.0) + idle * 2.0;
        let amount = on * star_amount / (1.0 + fi * 0.9);
        col += star_layer(p, t, amount);
    }

    return col;
}

// ── Nebula: transcript-faithful Blender short ────────────────────────────
// Noise texture 4D / FBM / normalize → color ramp (factor) → mix node with
// the base color (and the specular). Animated with a value node:
// `#frame * 0.01` (very slow W drift, with a multiplier for control).
// Stars: voronoi 4D → color ramp on the *distance* (spaced stops) → B slot
// and factor of the mix, with a multiply node before the voronoi W for star
// fade, and the size slider for star amount. View vector fed into both.

/// 3D voronoi F1: x = distance to nearest feature point, y = cell hash.
fn voronoi_f1(p: vec3<f32>) -> vec2<f32> {
    let base = floor(p);
    var best = 8.0;
    var best_hash = 0.0;
    for (var k = -1; k <= 1; k++) {
        for (var j = -1; j <= 1; j++) {
            for (var i = -1; i <= 1; i++) {
                let cell = base + vec3<f32>(f32(i), f32(j), f32(k));
                let rnd = hash31(cell);
                let feat = cell + vec3<f32>(
                    hash31(cell + 11.0),
                    hash31(cell + 27.0),
                    hash31(cell + 53.0),
                );
                let d = length(feat - p);
                if (d < best) {
                    best = d;
                    best_hash = rnd;
                }
            }
        }
    }
    return vec2<f32>(best, best_hash);
}

/// Video-faithful FBM: `detail` octaves blended, `roughness` amplitude
/// persistence, `lacunarity` frequency multiplier, `distortion` domain warp.
/// Matches the short's Noise Texture node (scale applied by the caller).
fn nebula_fbm(p: vec3<f32>, detail: f32, roughness: f32, lacunarity: f32, distortion: f32) -> f32 {
    // Domain warp (Distortion socket): offset the sample point by noise.
    let warp = vec3<f32>(
        vnoise3(p + vec3<f32>(0.0, 0.0, 0.0)),
        vnoise3(p + vec3<f32>(5.2, 1.3, 2.8)),
        vnoise3(p + vec3<f32>(8.3, 7.5, 9.1)),
    );
    let q = p + (warp - 0.5) * 2.0 * distortion;
    var n = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var total = 0.0;
    for (var i = 0; i < 8; i++) {
        let fi = f32(i);
        if (fi < detail) {
            let w = min(1.0, detail - fi);
            n += vnoise3(q * freq) * amp * w;
            total += amp * w;
            amp *= roughness;
            freq *= lacunarity;
        }
    }
    return n / max(total, 0.001);
}

/// Normalized FBM cloud through the video's Ease ramp: deep indigo →
/// purple → magenta at ~0.72 → hot pink tip. Palette-driven so skins tune
/// it: A = deep, B = mid purple, C = magenta.
fn nebula_cloud(q: vec3<f32>, w: f32, detail: f32, roughness: f32, lacunarity: f32, distortion: f32) -> vec3<f32> {
    let p = q + vec3<f32>(0.0, 0.0, w);
    let n = nebula_fbm(p, detail, roughness, lacunarity, distortion);
    // Normalize checkbox: remap the fbm band into 0..1.
    let norm = clamp((n - 0.30) * 2.4, 0.0, 1.0);
    // Ease ramp stops like the short (magenta pin ~0.72, hot tip at 1).
    var col = mix(vec3<f32>(0.010, 0.010, 0.060), ext.palette_a.rgb, smoothstep(0.0, 0.50, norm));
    col = mix(col, ext.palette_b.rgb, smoothstep(0.38, 0.72, norm));
    col = mix(col, ext.palette_c.rgb, smoothstep(0.62, 0.92, norm));
    col += vec3<f32>(1.0, 0.45, 0.85) * smoothstep(0.93, 1.0, norm) * 0.35;
    // Dust lanes rifted through.
    let dust = vnoise3(p * 3.31 - vec3<f32>(w));
    col *= 0.45 + 0.55 * smoothstep(0.30, 0.85, dust);
    return col;
}

/// Voronoi-distance stars through the video's tight ramp: white-hot cores at
/// distance 0, pink at ~0.11, black beyond — so only point centres glow.
/// Returns premasked star light (mask included), matching the short's
/// on-screen result (bright dots over purple cloud).
fn nebula_stars(p: vec3<f32>, w: f32, t: f32) -> vec3<f32> {
    let q = p + vec3<f32>(0.0, 0.0, w);
    let v = voronoi_f1(q);
    let d = v.x;
    let rnd = v.y;
    // Tight ramp: white core → palette magenta at 0.11 → black past ~0.25,
    // so only sparse ambient points glow and the clouds stay the focus.
    var scol = mix(vec3<f32>(1.0, 0.98, 0.97), ext.palette_c.rgb, smoothstep(0.0, 0.114, d));
    scol = mix(scol, vec3<f32>(0.02, 0.0, 0.03), smoothstep(0.114, 0.25, d));
    let mask = 1.0 - smoothstep(0.0, 0.25, d);
    let twinkle = 0.9 + 0.15 * sin(t * 1.6 + rnd * TAU);
    return scol * mask * twinkle;
}

/// Video-faithful nebula: cloud layers in the view-ray window with parallax,
/// then the voronoi stars mixed over the top. The short's 50/50 white Mix
/// node feeds Specular only in the final graph (its Base Color wire is
/// superseded by the star mix), so the base keeps the full cloud contrast
/// and the whitened cloud drives the roughness instead — no white wash.
/// Uniforms reuse the galaxy slots: scale/speed/stars/parallax in `galaxy`,
/// star_scale/star W rate/brightness in `galaxy_b`, noise knobs in
/// `nebula_b`. Returns the portal color; roughness is set by the caller.
fn nebula_effect(ray: vec3<f32>, t: f32) -> vec3<f32> {
    let scale = max(ext.galaxy.x, 0.05);
    let speed = ext.galaxy.y;
    let star_amount = ext.galaxy.z;
    let parallax = max(ext.galaxy.w, 0.0);
    let star_scale = max(ext.galaxy_b.x, 1.0);
    let star_rate = max(ext.galaxy_b.y, 0.0);
    let brightness = max(ext.galaxy_b.z, 0.0);
    let detail = max(ext.nebula_b.x, 1.0);
    let roughness = clamp(ext.nebula_b.y, 0.05, 1.0);
    let lacunarity = max(ext.nebula_b.z, 1.0);
    let distortion = max(ext.nebula_b.w, 0.0);
    // `#frame * 0.01` style W: cloud ×0.025, stars ×0.5 at 24fps.
    let cw = t * speed;
    let sw = t * star_rate;

    // Faint positional breathing only — the short animates W, not the
    // sample point, so a static camera sees a slow morph, not swimming.
    let idle = vec3<f32>(sin(t * 0.45) * 0.015, cos(t * 0.38) * 0.015, sin(t * 0.30) * 0.01);

    // Cloud: back-to-front layers with parallax; the view ray IS the
    // texture vector (Camera Data → Noise Vector in the short).
    var col = vec3<f32>(0.0);
    var total = 0.0;
    for (var i = 0u; i < 3u; i++) {
        let fi = f32(i);
        let depth = 1.0 + fi * (1.0 + parallax);
        let q = ray * depth * scale + vec3<f32>(fi * 37.0, fi * 19.0, 0.0) + idle * (1.0 + fi * 0.5);
        let weight = 1.0 / (1.0 + fi * 1.4);
        col += nebula_cloud(q, cw * (0.8 + fi * 0.35), detail, roughness, lacunarity, distortion) * weight;
        total += weight;
    }
    col /= max(total, 0.001);

    // Stars: voronoi F1 Euclidean over the same window, W faded by the
    // multiply node; nearer layers sparser.
    for (var i = 0u; i < 3u; i++) {
        let fi = f32(i);
        let depth = 1.0 + fi * (0.6 + parallax * 0.8);
        let p = ray * depth * star_scale + vec3<f32>(fi * 91.0, fi * 57.0, 0.0) + idle * 2.0;
        col += nebula_stars(p, sw, t) * star_amount / (1.0 + fi * 0.9);
    }

    return col * brightness;
}

/// Luminance helper for the specular hookup below.
fn portal_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// ── Matrix rain portal: classic cmatrix look ─────────────────────────────
// Dense vertical columns of falling glyphs on a pure black backdrop. Each
// column has its own phase, fall rate and brightness so neighbours desync;
// a white-hot head leads each stream with a long green tail decaying
// behind (above) it. Glyph cells are separated by dark borders so they read
// as distinct characters. Columns are strictly vertical (x only — no swirl)
// and the frame never moves, so all motion is the downward fall.
// Uniforms reuse the galaxy slots: density/speed/brightness/glyph_scale in
// `galaxy`, colour in `palette_a`.
fn matrix_effect(ray: vec3<f32>, t: f32) -> vec3<f32> {
    let density = max(ext.galaxy.x, 1.0);
    let speed = max(ext.galaxy.y, 0.01);
    let brightness = max(ext.galaxy.z, 0.0);
    let glyph_scale = max(ext.galaxy.w, 1.0);

    // View-ray window, unmoving: rain falls straight down through it.
    let q = ray * WINDOW_SCALE;

    // Columns: quantized across x only. Each gets its own fall rate, phase
    // and brightness; a few columns are bright streamers.
    let col_f = fract(q.x * density);
    let col_id = floor(q.x * density);
    let col_rnd = hash21(vec2<f32>(col_id, 7.0));
    let col_rnd2 = hash21(vec2<f32>(col_id, 31.0));
    let col_speed = speed * (0.5 + col_rnd * 1.0);
    let col_bright = 0.45 + col_rnd2 * 0.55;
    let streamer = step(0.93, col_rnd2);

    // Rows fall downward over time: a fixed glyph sits at lower y as t
    // grows, so the pattern visibly moves down the gun.
    let row_pos = q.y * glyph_scale + t * col_speed * glyph_scale * 0.35 + col_rnd * 37.0;
    let row_id = floor(row_pos);
    let head = fract(row_pos);
    let cell_rnd = hash21(vec2<f32>(col_id, row_id));
    let cell_rnd2 = hash21(vec2<f32>(col_id + 57.0, row_id));

    // Crisp separated glyph blocks: dark borders so neighbours read as
    // distinct characters instead of one green smear.
    let border = smoothstep(0.0, 0.10, col_f) * smoothstep(1.0, 0.90, col_f)
        * smoothstep(0.0, 0.12, head) * smoothstep(1.0, 0.88, head);

    // Blocky glyphs: most cells lit, some dark gaps, per-cell brightness
    // steps like changing characters. Flicker per glyph.
    let glyph_on = step(cell_rnd, 0.82);
    let glyph_tone = 0.55 + 0.45 * cell_rnd2;
    let flicker = 0.75 + 0.25 * sin(t * 7.0 + cell_rnd * TAU * 3.0);

    // White-hot leading (bottom) edge, long exponential tail above it —
    // the cmatrix signature for downward rain.
    let head_glow = smoothstep(0.18, 0.0, head);
    let trail = pow(1.0 - head, 2.2);

    let green = ext.palette_a.rgb;
    let white = vec3<f32>(0.9, 1.0, 0.92);
    var col = green * trail * 0.9 + white * head_glow;
    col *= glyph_on * glyph_tone * flicker * col_bright * border;
    col *= 1.0 + streamer * 1.5;

    // Pure black backdrop: the rain must pop, never wash.
    return col * brightness;
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
    // Grayscale sheets are binary high-contrast blobs (splinter/hazard sit
    // near 0.28/0.78), so a raw ×2.0 blows bright base colors out to white
    // (tiger orange hits 2.0 on red). Compress contrast around mid-grey
    // first: brights top out at 0.98× the base, so saturated bases keep
    // their hue and pattern instead of clipping (mathematically unclippable
    // for valid bases).
    const PATTERN_CONTRAST: f32 = 0.4;
    const PATTERN_GAIN: f32 = 1.6;
    let flat = clamp((stamped.rgb - 0.5) * PATTERN_CONTRAST + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    // Phantom-style: when the copy rolled a tint colourway, multiply by it
    // instead of the authored base.
    let pattern_tint = select(base.rgb, ext.tint.rgb, ext.extra.z > 0.5);
    base = vec4<f32>(mix(base.rgb, pattern_tint * flat * PATTERN_GAIN, strength), base.a);
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
        // depths add parallax. Mode 1 = galaxy, 2 = nebula, 3 = matrix.
        let ray = normalize(pbr_input.world_position.xyz - view.world_position);
        let t = ext.params.y * 0.35;
        var col: vec3<f32>;
        if (ext.params.w > 2.5) {
            col = matrix_effect(ray, t);
        } else if (ext.params.w > 1.5) {
            col = nebula_effect(ray, t);
        } else {
            col = galaxy_color(ray, t);
        }
        // All portals overwrite the base. The nebula's 50/50 white mix
        // feeds the specular response only (bright cloud → sharper
        // reflections), never the base color, so cloud contrast survives.
        base = vec4<f32>(col, base.a);
        pbr_input.material.metallic = 0.0;
        if (ext.params.w > 1.5 && ext.params.w < 2.5) {
            pbr_input.material.perceptual_roughness =
                clamp(0.55 - 0.35 * portal_luma(col), 0.15, 0.6);
        } else {
            pbr_input.material.perceptual_roughness = 0.6;
        }
        // Replace the authored emissive with the portal's own glow: mostly
        // deep space, with the dense knots and stars lighting themselves
        // softly.
        emissive = col * max(ext.anim.y, 0.15) * 0.35;
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
