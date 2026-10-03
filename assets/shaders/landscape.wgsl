// WARNING: full clanker code, gave me a quick background that I could tweak to what I wanted.
// I'm pretty sure there are so much more better ways to do this ?
// I'd like to write one at some point

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct LandscapeParams {
    mountain_lit: vec4<f32>,
    mountain_shadow: vec4<f32>,
    ridge_top: vec4<f32>,
    ridge_bottom: vec4<f32>,
    ground_lit: vec4<f32>,
    ground_shadow: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    aspect: f32,
    seed: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: LandscapeParams;

struct Range {
    freq: f32,      // big peaks per x unit (screen width = aspect)
    seed: f32,
    lo: f32,        // valley floor height
    hi: f32,        // summit height
    detail: f32,    // amplitude of small silhouette detail, relative to (hi - lo)
    wobble: f32,    // how much crest lines wander as they go down
    crease: f32,    // gully strength
}

//                   freq  seed  lo    hi    detail wobble crease
const FAR  = Range(3.0,  0.0,  0.55, 0.90, 0.2,  1.2,   0.4);
const MID  = Range(2.2,  7.0,  0.32, 0.50, 0.02,  4.0,   0.1);
const NEAR = Range(0.2, 13.0,  0.28, 0.32, 2.5,  0.5,   0.15);

// ---------------------------------------------------------------- noise

fn hash1(x: f32) -> f32 {
    return fract(sin(x * 127.1 + params.seed * 311.7) * 43758.5453);
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7)) + params.seed * 74.7) * 43758.5453);
}

fn noise1(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash1(i), hash1(i + 1.0), u);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i),                  hash2(i + vec2(1.0, 0.0)), u.x),
               mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x), u.y);
}

/// 1D ridged fbm in [0, 1]: sharp summits, rounded valleys, one continuous chain.
fn ridge1(x: f32, octaves: i32) -> f32 {
    var s = 0.0; var a = 0.5; var norm = 0.0; var q = x;
    for (var i = 0; i < octaves; i++) {
        s += a * (1.0 - abs(noise1(q) * 2.0 - 1.0));
        norm += a;
        q = q * 2.03 + 5.1;
        a *= 0.5;
    }
    return s / norm;
}

/// 2D ridged fbm: sharp creases -> gullies on the faces.
fn ridge2(p: vec2<f32>) -> f32 {
    var s = 0.0; var a = 0.5; var q = p;
    for (var i = 0; i < 4; i++) {
        s += a * (1.0 - abs(noise2(q) * 2.0 - 1.0));
        q = q * 2.03 + 3.7;
        a *= 0.5;
    }
    return s;
}

// ---------------------------------------------------------------- mountains

/// Large-scale shape of the chain (drives the lighting).
fn base_shape(x: f32, r: Range) -> f32 {
    return ridge1(x * r.freq + r.seed, 2);
}

/// Full silhouette = base shape + small detail.
fn silhouette(x: f32, r: Range) -> f32 {
    let b = base_shape(x, r);
    let d = ridge1(x * r.freq * 5.0 + r.seed * 2.7 + 11.0, 3) - 0.5;
    return mix(r.lo, r.hi, b + r.detail * d);
}

fn shade_range(x: f32, y: f32, r: Range, lit: vec3<f32>, shadow: vec3<f32>, col_in: vec3<f32>) -> vec3<f32> {
    let sil = silhouette(x, r);
    let below = max(sil - y, 0.0);

    // Crest lines start at summits and wander as they go down:
    // sample the slope at a noise-shifted x instead of at x itself.
    let xs = x + (noise1(y * 9.0 + r.seed * 4.0) - 0.5) * r.wobble * below;

    // Slope of the big shape. Rising to the right = face turned left = lit.
    let e = 0.003;
    let slope = (base_shape(xs + e, r) - base_shape(xs - e, r)) / (2.0 * e);
    var side = -slope / r.freq;          // ~[-1, 1], > 0 means shadow

    // Gullies: x-slope of 2D ridged noise, growing with depth.
    let p = vec2(x, y) * 7.0 + vec2(r.seed * 3.3, r.seed);
    let dcx = (ridge2(p + vec2(e, 0.0)) - ridge2(p - vec2(e, 0.0))) / (2.0 * e);
    side += dcx * r.crease * below;

    // Toon split, light from the left (negate `side` for light from the right).
    let aa = max(fwidth(side), 1e-4);
    let c = mix(lit, shadow, smoothstep(-aa, aa, side));

    let edge = max(fwidth(y - sil), 1e-4);
    let inside = 1.0 - smoothstep(-edge, edge, y - sil);
    return mix(col_in, c, inside);
}

fn mist(col: vec3<f32>, fog: vec3<f32>, y: f32, base: f32, height: f32, strength: f32) -> vec3<f32> {
    return mix(col, fog, (1.0 - smoothstep(base, base + height, y)) * strength);
}

// ---------------------------------------------------------------- fragment

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = in.uv.x * params.aspect;
    let side_dist = abs(in.uv.x * 2.0 - 1.0);
    let y = (1.0 - in.uv.y) / mix(0.8, 1.5, pow(side_dist, 1.8));

    let horizon = params.sky_horizon.rgb;
    let fog = mix(horizon, vec3(1.0), 0.4);

    var col = mix(horizon, params.sky_top.rgb, smoothstep(0.3, 1.0, y));

    let far_lit    = mix(params.mountain_lit.rgb,    horizon, 0.15);
    let far_shadow = mix(params.mountain_shadow.rgb, horizon, 0.25);
    col = shade_range(x, y, FAR, far_lit, far_shadow, col);
    col = mist(col, fog, y, 0.40, 0.40, 1.75);

    col = shade_range(x, y, MID, params.ridge_top.rgb, params.ridge_bottom.rgb, col);
    col = mist(col, fog, y, 0.17, 0.15, 0.2);

    col = shade_range(x, y, NEAR, params.ground_lit.rgb, params.ground_shadow.rgb, col);

    return vec4(col, 1.0);
}
