// WARNING: full clanker code, gave me a quick background that I could tweak to what I wanted.
// I'm pretty sure there are so much more better ways to do this ?
// I'd like to write one at some point

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct LandscapeParams {
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    sun: vec4<f32>,
    cloud: vec4<f32>,
    mountain: vec4<f32>,
    snow: vec4<f32>,
    hill_far: vec4<f32>,
    hill_near: vec4<f32>,
    ground: vec4<f32>,
    ground_dark: vec4<f32>,
    accent: vec4<f32>,
    petal: vec4<f32>,
    aspect: f32,
    seed: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: LandscapeParams;

const GROUND: f32 = 0.2; // must match FLOOR_TOP: 15% of the screen height

// ─── Noise ───────────────────────────────────────────────────────────────────

fn hash1(x: f32) -> f32 {
    return fract(sin(x * 127.1 + params.seed * 311.7) * 43758.5453);
}

fn hash2(v: vec2<f32>) -> f32 {
    return fract(sin(dot(v, vec2(127.1, 311.7)) + params.seed) * 43758.5453);
}

fn noise1(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash1(i), hash1(i + 1.0), u);
}

fn noise2(v: vec2<f32>) -> f32 {
    let i = floor(v);
    let f = fract(v);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x),
        mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x),
        u.y,
    );
}

/// Layered 1D noise, normalized to [0, 1]
fn fbm1(x: f32, octaves: i32) -> f32 {
    var sum = 0.0;
    var norm = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    for (var i = 0; i < octaves; i++) {
        sum += amp * noise1(x * freq);
        norm += amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    return sum / norm;
}

/// Layered 2D noise, roughly [0, 1]
fn fbm2(v: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = v;
    for (var i = 0; i < 4; i++) {
        sum += amp * noise2(q);
        q *= 2.0;
        amp *= 0.5;
    }
    return sum / 0.9375;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Signed distance to an axis-aligned box of half-size b, centered on the origin
fn sdbox(q: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = abs(q) - b;
    return length(max(d, vec2(0.0))) + min(max(d.x, d.y), 0.0);
}

/// Atmospheric perspective: distant things fade toward the horizon color
fn haze(col: vec3<f32>, amount: f32) -> vec3<f32> {
    return mix(col, params.sky_horizon.rgb, clamp(amount, 0.0, 1.0));
}

/// A wispy mist band, densest below `base`, drifting sideways with the wind
fn mist(col: vec3<f32>, x: f32, y: f32, base: f32, strength: f32, speed: f32) -> vec3<f32> {
    let t = globals.time;
    let wisps = fbm2(vec2(x * 2.5 - t * speed, y * 10.0 + sin(t * 0.05) * 0.3));
    let amount = smoothstep(base + 0.05, base - 0.10, y) * (0.5 + 0.5 * wisps) * strength;
    return mix(col, params.sky_horizon.rgb, clamp(amount, 0.0, 1.0));
}

/// Fuji-like volcano: concave slopes, a touch of noise on the ridge
fn volcano(x: f32, cx: f32) -> f32 {
    let d = abs(x - cx);
    return 0.64 - 0.52 * pow(d / 1.1, 0.72) + 0.008 * (noise1(x * 14.0) - 0.5);
}

/// Drifting cherry petals: one petal per random cell, falling and swaying
fn petals(x: f32, y: f32, t: f32, scale: f32, speed: f32, size: f32) -> f32 {
    let q = vec2(x + 0.06 * sin(t * 0.8 + y * 4.0), y + t * speed);
    let cell = floor(q * scale);
    let f = fract(q * scale);
    if (hash2(cell + 0.37) < 0.75) {
        return 0.0;
    }
    let c = vec2(hash2(cell + 1.3), hash2(cell + 5.7)) * 0.6 + 0.2;
    let d = length((f - c) * vec2(1.0, 1.6));
    return smoothstep(size, size * 0.6, d);
}

// ─── Scene ───────────────────────────────────────────────────────────────────

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let y = 1.0 - in.uv.y;            // 0 = bottom, 1 = top
    let x = in.uv.x * params.aspect;  // 0 .. aspect, so noise isn't stretched
    let cx = params.aspect * 0.5;     // screen center

    // Sky: warm at the horizon, deep at the top, with faint haze bands
    var col = mix(params.sky_horizon.rgb, params.sky_top.rgb, smoothstep(0.12, 1.0, y));
    col = mix(col, params.sky_horizon.rgb, 0.08 * fbm2(vec2(x * 0.8, y * 6.0)));

    // Sun: big, low, right of center, with a soft glow
    let sun_pos = vec2(cx + 0.55, 0.50);
    let sun_d = length(vec2(x, y) - sun_pos);
    col += params.sun.rgb * exp(-sun_d * 4.0) * 0.35;
    col = mix(col, params.sun.rgb, smoothstep(0.14, 0.133, sun_d));

    // Clouds: thin streaks drifting slowly in front of the sun
    let band = smoothstep(0.50, 0.60, y) * smoothstep(0.90, 0.72, y);
    let cloud = smoothstep(0.55, 0.72, fbm2(vec2(x * 1.2 - t * 0.012, y * 7.0))) * band;
    col = mix(col, params.cloud.rgb, cloud * 0.8);

    // The volcano, far away: hazed, snow above a jagged snow line
    let vx = cx - 0.25;
    let v = volcano(x, vx);
    if (y < v) {
        let snow_line = 0.46 + 0.04 * noise1(x * 20.0 + 7.0);
        let rock = haze(params.mountain.rgb, 0.45 * (1.0 - y));
        let snow = haze(params.snow.rgb, 0.25);
        col = mix(rock, snow, smoothstep(snow_line - 0.01, snow_line + 0.01, y));
    }
    col = mist(col, x, y, 0.40, 0.7, 0.010);

    // Far hills
    let h1 = 0.30 + 0.10 * fbm1(x * 1.6 + 5.0, 4);
    if (y < h1) {
        let shade = smoothstep(h1, h1 - 0.25, y);
        col = haze(mix(params.hill_far.rgb, params.hill_far.rgb * 0.8, shade), 0.3);
    }
    col = mist(col, x, y, 0.30, 0.6, 0.018);

    // Near hills
    let h2 = 0.22 + 0.07 * fbm1(x * 2.4 + 23.0, 4);
    if (y < h2) {
        let shade = smoothstep(h2, h2 - 0.2, y);
        col = haze(mix(params.hill_near.rgb, params.hill_near.rgb * 0.75, shade), 0.12);
    }
    col = mist(col, x, y, 0.20, 0.8, 0.028);

    // Torii gate on the left, standing on the ground, lightly hazed
    let tx = cx - 0.78;
    let pillars = min(
        sdbox(vec2(x - (tx - 0.10), y - 0.29), vec2(0.014, 0.14)),
        sdbox(vec2(x - (tx + 0.10), y - 0.29), vec2(0.014, 0.14)),
    );
    let nuki = sdbox(vec2(x - tx, y - 0.37), vec2(0.17, 0.011));
    let kasagi_y = 0.445 + 0.025 * pow(abs(x - tx) / 0.22, 2.0); // top beam curves up at the ends
    let kasagi = sdbox(vec2(x - tx, y - kasagi_y), vec2(0.22, 0.016));
    let gakuzuka = sdbox(vec2(x - tx, y - 0.405), vec2(0.02, 0.02));
    let torii = 1.0 - smoothstep(0.0, 0.003, min(min(pillars, nuki), min(kasagi, gakuzuka)));
    col = mix(col, haze(params.accent.rgb, 0.2), torii);

    // Ground: raked sand, a lighter fighting platform in the middle, darker toward the viewer
    if (y < GROUND) {
        let depth = (GROUND - y) / GROUND; // 0 at the horizon line, 1 at the bottom
        var g = mix(params.ground.rgb, params.ground_dark.rgb, depth * 0.8);
        let rake = 0.5 + 0.5 * sin(y * 500.0 + 3.0 * fbm2(vec2(x * 4.0, y * 30.0)));
        g = mix(g, g * 0.9, smoothstep(0.6, 0.9, rake) * 0.6);
        let platform = smoothstep(1.1, 0.9, abs(x - cx));
        g = mix(g, g * 1.1, platform * 0.5);
        g = mix(g, params.snow.rgb * 0.5 + g * 0.5, smoothstep(0.012, 0.0, GROUND - y)); // lit edge
        col = g;
        col = mix(col, params.sky_horizon.rgb, smoothstep(0.3, 0.0, depth) * 0.35); // haze at the far edge
    }

    // Cherry petals: a near layer and a far, fainter one
    let near = petals(x, y, t, 6.0, 0.14, 0.10);
    let far = petals(x, y, t, 11.0, 0.08, 0.07) * 0.6;
    col = mix(col, params.petal.rgb, max(near, far) * 0.9);

    // Vignette
    let vig = length((in.uv - 0.5) * vec2(1.0, 1.2));
    col *= 1.0 - 0.3 * smoothstep(0.45, 0.85, vig);

    return vec4(clamp(col, vec3(0.0), vec3(1.0)), 1.0);
}
