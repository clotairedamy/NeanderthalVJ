// The particle head, ported from BUST_VERT / BUST_FRAG in
// `visualizer/viz/mode_pointcloud.py`.
//
// One departure from the GLSL, forced by the API: WebGPU has no
// `gl_PointSize`, so each particle is expanded into a camera-facing quad
// (six vertices per particle, no instance buffer). The fragment shader then
// cuts it to a round dot, which is what the GL point sprite gave for free.
//
// The particle's own work -- deform, dissolve, light -- runs once in
// `cs_particles`, which writes a sprite per particle. Done in the vertex
// shader it ran six times over, once for each corner of the quad.

struct Uniforms {
    view_proj : mat4x4<f32>,
    pivot     : vec3<f32>,
    yaw       : f32,
    shake     : vec3<f32>,
    pitch     : f32,
    tint      : vec3<f32>,
    tint_mix  : f32,
    glitch    : f32,
    smear     : f32,
    dust      : f32,
    seed      : f32,
    bands     : f32,
    time      : f32,
    bright    : f32,
    point     : f32,
    low       : f32,
    mid       : f32,
    high      : f32,
    swell     : f32,
    explode   : f32,
    fade      : f32,
    dissipate : f32,
    diss_t    : f32,
    // the viewport in pixels: the GL original sized its sprites in
    // pixels, so the conversion to clip space needs both dimensions
    vp_w      : f32,
    vp_h      : f32,
    // the bands laid along an axis of the head (0 off, 1 vertical,
    // 2 depth, 3 horizontal, 4 radial), and how far each region lifts
    band_axis : f32,
    band_push : f32,
    // up to three ripples travelling out from the neck:
    // (radius, strength, width, unused). vec4 so each aligns to 16 and
    // the Rust array lines up without further padding.
    wave0     : vec4<f32>,
    wave1     : vec4<f32>,
    wave2     : vec4<f32>,
    // the camera, for dropping the far side of the head
    eye       : vec4<f32>,
    // (the rings' centre in world x, z; the song's intensity; the flash
    // of a drop)
    meter     : vec4<f32>,
    // Synth Cloud's sheet of light: (zone, width, strength, shimmer phase)
    sheet     : vec4<f32>,
    // the three newest rings: (radius, fade, width, how much they carry)
    rim0      : vec4<f32>,
    rim1      : vec4<f32>,
    rim2      : vec4<f32>,
    // the Depth Point Cloud's head effect -- see head_fx.rs: (which,
    // amount, the beat, time), (lows, mids, highs, alive) and its paths
    fx        : vec4<f32>,
    fx2       : vec4<f32>,
    paths     : array<vec4<f32>, 14>,
    // the key light, turned with the camera
    key       : vec4<f32>,
};

@group(0) @binding(0) var<uniform> u : Uniforms;

struct Particle {
    pos  : vec3<f32>,
    zone : f32,
    nrm  : vec3<f32>,
    fade : f32,
    rnd  : f32,
    // three scalars, not a vec3: a trailing vec3 aligns to 16 and makes
    // this struct 64 bytes where the Rust one is 48, so every particle
    // after the first would be read from the wrong offset
    _pad0 : f32,
    _pad1 : f32,
    _pad2 : f32,
};

@group(0) @binding(1) var<storage, read> particles : array<Particle>;

// A particle after the compute pass: its centre in clip space, its radius
// (pixels times clip.w, so the vertex shader only divides by the
// viewport) and its colour, packed -- the target is 8-bit, so nothing is
// lost. 32 bytes; `SPRITE_BYTES` in renderer.rs must agree.
struct Sprite {
    clip   : vec4<f32>,
    radius : f32,
    color  : u32,
    _pad0  : f32,
    _pad1  : f32,
};

@group(1) @binding(0) var<storage, read_write> sprites_out : array<Sprite>;
@group(1) @binding(1) var<storage, read> sprites : array<Sprite>;

fn hash(n : f32) -> f32 { return fract(sin(n) * 43758.5453123); }
fn hash3(c : vec3<f32>) -> f32 { return hash(dot(c, vec3<f32>(1.0, 57.0, 113.0))); }

fn rot_z(a : f32) -> mat3x3<f32> {
    let c = cos(a); let s = sin(a);
    return mat3x3<f32>(vec3<f32>(c, s, 0.0), vec3<f32>(-s, c, 0.0), vec3<f32>(0.0, 0.0, 1.0));
}
fn rot_x(a : f32) -> mat3x3<f32> {
    let c = cos(a); let s = sin(a);
    return mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, c, s), vec3<f32>(0.0, -s, c));
}

// Band pushes, slice twist/slide and the head's turn on the neck: shared by
// the particles and the occluding shell so the two never separate.
fn band_w_low(z : f32) -> f32 { return 1.0 - smoothstep(0.30, 0.45, z); }
fn band_w_mid(z : f32) -> f32 {
    return smoothstep(0.50, 0.60, z) * (1.0 - smoothstep(0.76, 0.86, z));
}

struct Deformed {
    p    : vec3<f32>,
    n    : vec3<f32>,
    calm : f32,
    h2   : f32,
};

/// One ripple's push at distance `d` from where it started.
fn ripple(w : vec4<f32>, d : f32) -> f32 {
    if (w.y <= 0.0001) { return 0.0; }
    let x = (d - w.x) / max(w.z, 1e-4);
    // a travelling crest, not a step: the surface lifts as the front
    // passes and settles behind it
    return exp(-x * x) * w.y;
}

/// Where a point sits along the chosen band axis, 0..1 (bass to treble),
/// and how strongly its band is playing.
fn band_light(pos : vec3<f32>, zone : f32) -> f32 {
    let axis = i32(u.band_axis + 0.5);
    if (axis == 0) { return 0.0; }
    let rel = pos - u.pivot;
    var t = zone;
    if (axis == 2) { t = clamp(0.5 - rel.y * 0.6, 0.0, 1.0); }        // face front to back of the skull
    if (axis == 3) { t = clamp(0.5 + rel.x * 0.6, 0.0, 1.0); }        // ear to ear
    if (axis == 4) { t = clamp(length(rel) * 0.7, 0.0, 1.0); }        // centre outwards
    let lo_mid = mix(u.low, u.mid, smoothstep(0.0, 0.5, t));
    return mix(lo_mid, u.high, smoothstep(0.5, 1.0, t));
}

fn deform(p_in : vec3<f32>, n_in : vec3<f32>, zone : f32) -> Deformed {
    var p = p_in;
    var n = n_in;
    // A hairline swell on the neck and shoulders only -- never the face.
    // Pushing the whole surface along its normals inflates the nose and
    // the eye sockets, which reads as damage however small it is.
    p = p + n * 0.012 * u.low * band_w_low(zone);

    let band = floor(zone * u.bands + hash(u.seed) * 0.999);
    let h1 = hash(band * 12.9898 + u.seed);
    let h2 = hash(band * 78.233 + u.seed * 1.7);
    let calm = step(0.35, h1) * smoothstep(0.25, 0.40, zone);
    let twist = u.glitch * (h1 * 2.0 - 1.0) * 0.62 * calm;
    let slide = u.glitch * (h2 * 2.0 - 1.0) * 0.14 * calm;

    // the whole head moves as one; only the neck bends
    let hw = smoothstep(0.16, 0.36, zone);
    let R = rot_x(-u.pitch * hw) * rot_z(u.yaw * hw + twist);
    p = u.pivot + R * (p - u.pivot) * (1.0 + u.swell * hw + 0.55 * u.explode);
    n = R * n;
    p.x = p.x + slide;
    p = p + u.shake;            // the whole bust rattles as one

    var out : Deformed;
    out.p = p; out.n = n; out.calm = calm; out.h2 = h2;
    return out;
}

/// A thin band of light around the head at height `centre`.
fn contour(zone : f32, centre : f32, width : f32) -> f32 {
    let x = (zone - centre) / max(width, 1e-4);
    return exp(-x * x);
}

// ─── the Depth Point Cloud's head effects (head_fx.rs) ──────────────────
//
// The face is never touched. An effect borrows only particles the camera
// cannot see -- the far side of the head, behind the shell -- and flies
// them around the head; and anything it would carry in front of the face
// is dropped before it can cover it.

const TAU : f32 = 6.2831853;

/// Where comet `i` is on its Lissajous path, `back` radians ago.
fn lissajous(i : i32, back : f32) -> vec3<f32> {
    let r = u.paths[i];
    let phase = u.paths[3 + i].xyz;
    let dir = normalize(sin(r.xyz * (r.w - back) + phase) + vec3<f32>(0.0, 0.0, 1e-3));
    return u.paths[6].xyz + dir * u.paths[7].xyz * u.paths[6].w;
}

/// The superformula's radius at angle `th`: circles, polygons, stars and
/// flowers from four numbers.
fn superformula(th : f32, m : f32, n1 : f32, n2 : f32) -> f32 {
    let a = pow(max(abs(cos(m * th * 0.25)), 1e-4), n2);
    let b = pow(max(abs(sin(m * th * 0.25)), 1e-4), n2);
    return pow(max(a + b, 1e-4), -1.0 / n1);
}

/// One of the seven bands, as the corona reads them.
fn band_level(i : i32) -> f32 {
    let v = select(u.paths[13], u.paths[12], i < 4);
    let k = i % 4;
    if (k == 0) { return v.x; }
    if (k == 1) { return v.y; }
    if (k == 2) { return v.z; }
    return v.w;
}

/// 1 well clear of the face, 0 in front of it: how much of a borrowed
/// particle may be drawn where it is. Only the face is kept clear: an
/// effect may pass in front of the back or the sides of the head, so a
/// vortex seen from behind still wraps all the way round.
fn clear_of_face(p : vec3<f32>) -> f32 {
    let c = u.paths[6].xyz;
    let radii = u.paths[7].xyz;
    let toward = normalize(u.eye.xyz - c);
    let v = p - c;
    let front = dot(v, toward);
    // behind the head's middle, the shell hides whatever it overlaps
    if (front <= 0.0) { return 1.0; }
    // the model faces -y, turned by the head's yaw
    let face = vec3<f32>(sin(u.yaw), -cos(u.yaw), 0.0);
    if (dot(v, face) <= 0.0) { return 1.0; }
    let side = normalize(cross(toward, vec3<f32>(0.0, 0.0, 1.0)));
    let up = cross(side, toward);
    let off = v - toward * front;
    let q = vec2<f32>(dot(off, side) / radii.x, dot(off, up) / radii.z);
    return smoothstep(1.05, 1.35, length(q));
}

/// Whether `p` lies behind the head as the camera sees it, where the head
/// would hide it: such a particle is not drawn at all, rather than drawn
/// and then covered.
fn hidden_by_head(p : vec3<f32>) -> bool {
    let c = u.paths[6].xyz;
    let radii = u.paths[7].xyz;
    let toward = normalize(u.eye.xyz - c);
    let v = p - c;
    if (dot(v, toward) > 0.0) { return false; }
    let side = normalize(cross(toward, vec3<f32>(0.0, 0.0, 1.0)));
    let up = cross(side, toward);
    let q = vec2<f32>(dot(v, side) / radii.x, dot(v, up) / radii.z);
    // a little inside the outline, so nothing at the edge is lost
    return length(q) < 0.85;
}

/// A borrowed particle: where it goes and how bright it is. Brightness 0
/// leaves the particle where it was.
/// `surf` and `n` are where the borrowed particle sits on the head, and
/// its normal; `rest` is that point before the head moved, and `zone` its
/// height.
fn effect_dot(rnd : f32, surf : vec3<f32>, n : vec3<f32>, rest : vec3<f32>, zone : f32) -> vec4<f32> {
    let which = i32(u.fx.x + 0.5);
    let amount = u.fx.y;
    let kick = u.fx.z;
    let time = u.fx.w;
    let lows = u.fx2.x;
    let mids = u.fx2.y;
    let highs = u.fx2.z;
    let alive = u.fx2.w;
    let c = u.paths[6].xyz;
    let radii = u.paths[7].xyz;
    let j = (vec3<f32>(hash(rnd * 11.3), hash(rnd * 23.9), hash(rnd * 37.1)) - 0.5) * 2.0;
    let pick = hash(rnd * 733.1);

    var p = vec3<f32>(0.0);
    var light = 0.0;
    if (which == 1) {
        // Orbits: a share of the hidden particles become three comets
        if (pick > 0.10) { return vec4<f32>(0.0); }
        let i = i32(min(hash(rnd * 41.3) * 3.0, 2.0));
        // more grains toward the comet's head than its tail
        let along = sqrt(hash(rnd * 59.7));
        let width = (0.02 + 0.10 * (1.0 - along)) * (1.0 + 0.6 * lows + 0.8 * kick);
        p = lissajous(i, (1.0 - along) * u.paths[7].w) + j * width;
        light = (0.25 + 1.75 * along * along) * (0.6 + 0.8 * lows + kick);
        // the highs make the tails glitter
        light = light * (1.0 + 2.0 * highs * step(0.9, fract(rnd * 17.3 + time * 3.1)));
    } else if (which == 2) {
        // Halo: a superformula crown behind the head
        if (pick > 0.16) { return vec4<f32>(0.0); }
        let h = u.paths[8];
        let th = hash(rnd * 17.9) * TAU;
        // the highs sharpen it into points, the mids round its sides
        let n1 = 0.6 + 2.4 * (1.0 - highs);
        let n2 = 1.0 + 0.6 * mids;
        let r = clamp(mix(superformula(th, h.x, n1, n2), superformula(th, h.y, n1, n2), h.z), 0.75, 1.5);
        var big = max(radii.x, radii.z) * (1.25 + 0.3 * lows + 0.2 * kick);
        light = 0.5 + 0.9 * lows + 0.5 * kick;
        // a quarter of it rides the ring each beat sends out
        if (hash(rnd * 7.7) < 0.25) {
            big = big * (1.0 + 1.1 * (1.0 - kick));
            light = light * kick * 1.4;
        }
        let toward = normalize(u.eye.xyz - c);
        let side = normalize(cross(toward, vec3<f32>(0.0, 0.0, 1.0)));
        let up = cross(side, toward);
        let a = th + h.w;
        p = c - toward * 0.35 + (side * cos(a) + up * sin(a)) * r * big
          + j * (0.012 + 0.05 * highs);
    } else if (which == 3) {
        // Vortex: three arms spiralling up around the head
        if (pick > 0.12) { return vec4<f32>(0.0); }
        let v = u.paths[9];
        let arm = floor(hash(rnd * 3.1) * 3.0);
        let f = fract(hash(rnd * 91.1) + v.y);
        let z = c.z + (f - 0.5) * 2.0 * radii.z;
        // every particle turns at the arm's speed: a spread of speeds
        // smears the arms into a curtain within seconds
        let rad = (1.3 + 0.08 * hash(rnd * 33.3)) * (1.0 + 0.2 * lows + 0.15 * kick);
        let a = arm * TAU / 3.0 + v.x + (z - c.z) * 2.6 + (hash(rnd * 5.9) - 0.5) * 0.12;
        p = vec3<f32>(c.x + cos(a) * rad * radii.x, c.y + sin(a) * rad * radii.y, z) + j * 0.012;
        // fading in at the bottom and out at the top
        light = (0.35 + 0.8 * mids + 0.6 * kick) * sin(3.14159 * f);
    } else if (which == 4) {
        // Corona: flares grow out of the head itself. Each borrowed
        // particle is a point on the far side of the skull; its flare
        // leaves along that point's own normal, so the fire follows the
        // real shape of the head and turns with it. Seen from the front,
        // only the flares leaving past the outline show -- a corona.
        if (pick > 0.9 || zone < 0.42) { return vec4<f32>(0.0); }
        // the spectrum up the head: bass at the neck, highs at the crown
        let slot = clamp((zone - 0.42) / 0.58, 0.0, 1.0) * 6.0;
        let bi = i32(floor(slot));
        let level = mix(band_level(bi), band_level(min(bi + 1, 6)), slot - f32(bi));
        // particles from the same patch of skull make one tongue of fire,
        // which is what makes flames rather than fuzz
        let piece = floor(rest * 7.0);
        let tongue = hash3(piece);
        // a third of the tongues stay low and dim, so the rest stand out as
        // rays with dark between them, not an even fuzz
        let ray = step(0.35, hash(tongue * 13.7));
        let jag = 0.5 + 0.9 * hash(tongue * 91.7 + floor(time * 6.0) * 0.37);
        let len = (0.15 + (1.8 * level * jag + 1.0 * kick) * (0.25 + 0.75 * ray)) * amount;
        let lean = vec3<f32>(hash(tongue * 3.1), hash(tongue * 5.3), hash(tongue * 7.9)) - 0.5;
        // out from the head as the camera sees it, as well as along the
        // skull's own normal: a flare from the back of the head fans out
        // past the outline instead of hiding straight behind it
        let toward = normalize(u.eye.xyz - c);
        let off = surf - c;
        let fan = normalize(off - toward * dot(off, toward) + vec3<f32>(0.0, 0.0, 1e-3));
        var dir = normalize(n * 0.6 + fan * 1.1 + vec3<f32>(0.0, 0.0, 0.35) + lean * 0.45);
        // never toward the camera: from the far side of the head, that
        // keeps every flare behind the face
        dir = normalize(dir - toward * max(dot(dir, toward), 0.0) + vec3<f32>(0.0, 0.0, 1e-4));
        // more of it near the root, where the fire is hottest, and a sway
        // that runs up each tongue
        let s = pow(hash(rnd * 59.7), 1.6);
        let sway = normalize(cross(dir, vec3<f32>(0.0, 0.0, 1.0)) + vec3<f32>(1e-4, 0.0, 0.0));
        // each tongue narrows to a point: its particles close in on the
        // middle of their patch as they rise
        let taper = (rest - (piece + 0.5) / 7.0) * min(s * 1.6, 0.92);
        p = surf - taper + dir * s * len
          + sway * sin(s * 8.0 - time * 7.0 + tongue * 20.0) * 0.06 * s
          + j * 0.006;
        light = (0.3 + 0.9 * (1.0 - s)) * (0.4 + 0.8 * level + 0.3 * kick) * (0.2 + 0.8 * ray);
        // the highs make the tips spit sparks
        light = light * (1.0 + 2.5 * highs * s * step(0.94, fract(rnd * 13.1 + time * 4.3)));
        // it starts on the far side and only moves away from the camera,
        // so the head's own shell hides whatever lies behind the face
        if (hidden_by_head(p)) { return vec4<f32>(p, -1.0); }
        return vec4<f32>(p, light * amount * alive);
    } else if (which == 5) {
        // Supernova: shells of light blasted out from the head, each the
        // head's own shape inflating from its surface
        if (zone < 0.42) { return vec4<f32>(0.0); }
        let k = i32(min(hash(rnd * 41.3) * 4.0, 3.0));
        let pair = u.paths[10 + k / 2];
        var shell = pair.xy;
        if (k % 2 == 1) { shell = pair.zw; }
        if (shell.y <= 0.02 || pick > 0.6) { return vec4<f32>(0.0); }
        // a shell with depth: the leading edge brightest, a wake behind it
        let lag = pow(hash(rnd * 61.3), 3.0) * 0.18 * shell.x;
        p = c + (surf - c) * (1.02 + shell.x - lag) + j * 0.01;
        let edge = 1.0 - lag / max(0.18 * shell.x, 1e-3);
        light = shell.y * (0.3 + 1.3 * edge) * (1.0 - shell.x / 6.0);
        if (hidden_by_head(p)) { return vec4<f32>(p, -1.0); }
    } else {
        return vec4<f32>(0.0);
    }
    // a dot is opaque, so a dimmed one would show as a dark speck over the
    // face: anything near it is dropped outright instead
    if (clear_of_face(p) < 0.5) { return vec4<f32>(p, -1.0); }
    return vec4<f32>(p, light * amount * alive);
}

/// The sprite for a borrowed particle, lit like the rest of the scene.
fn effect_sprite(p : vec3<f32>, light : f32, rnd : f32) -> Sprite {
    var sp : Sprite;
    let clip = u.view_proj * vec4<f32>(p, 1.0);
    let lum = light * u.meter.z * (1.0 + u.meter.w);
    let col = vec3<f32>(lum) * mix(vec3<f32>(1.0), u.tint, u.tint_mix);
    let px = u.point * (0.6 + 0.8 * hash(rnd * 53.9)) * clamp(7.0 / max(clip.w, 0.001), 0.5, 2.5);
    sp.clip = clip;
    sp.radius = px * clip.w;
    sp.color = pack4x8unorm(vec4<f32>(col, 1.0));
    if (light <= 0.004) {
        sp.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        sp.radius = 0.0;
    }
    return sp;
}

@compute @workgroup_size(256)
fn cs_particles(@builtin(global_invocation_id) gid : vec3<u32>) {
    let pi = gid.x;
    if (pi >= arrayLength(&particles)) { return; }
    let part = particles[pi];

    var p = part.pos;
    var n = part.nrm;
    let zone = part.zone;
    let a_rnd = part.rnd;

    // each band lives in its own part of the body
    let w_low  = band_w_low(zone);
    let w_mid  = band_w_mid(zone);
    let w_high = smoothstep(0.84, 0.98, zone);

    let d = deform(p, n, zone);
    p = d.p; n = d.n;

    // A head effect borrows particles the camera cannot see: those on the
    // far side of the head, behind the shell. The face's own particles go
    // on below exactly as they would without it.
    if (u.fx.x > 0.5 && dot(n, normalize(u.eye.xyz - p)) < -0.4) {
        let e = effect_dot(a_rnd, p, n, part.pos, zone);
        if (e.w != 0.0) {
            sprites_out[pi] = effect_sprite(e.xyz, max(e.w, 0.0), a_rnd);
            return;
        }
    }

    // each region along the band axis answers its own band: with light
    // always, and lifted off the surface only as far as Band push asks
    let band_e = band_light(part.pos, zone);
    p = p + n * band_e * u.band_push * 0.09;

    // a share of the particles streak sideways, as ghost trails
    let dir = select(1.0, -1.0, d.h2 < 0.5);
    let streaker = step(0.70, hash(a_rnd * 3.3));
    let streak = u.smear * dir * pow(fract(a_rnd * 3.7), 2.0) * 0.8 * d.calm * streaker;
    p.x = p.x + streak;

    // grazing surfaces dissolve: silhouette particles loosen into grains.
    // Not on the front of the face, though: the eye sockets and the lids
    // face sideways too, and loosening them broke the far eye into a
    // smudge.
    let facing = abs(n.y);
    let edge = pow(1.0 - facing, 3.0);
    let face = smoothstep(0.50, 0.58, zone) * (1.0 - smoothstep(0.86, 0.94, zone))
             * smoothstep(0.15, 0.45, u.pivot.y - part.pos.y);
    let jit = vec3<f32>(hash(a_rnd * 17.1), hash(a_rnd * 29.3), hash(a_rnd * 41.7)) - 0.5;
    p = p + (n * hash(a_rnd * 9.7) * 0.05 + jit * 0.035) * edge * (1.0 - face);

    // the explosion throws each particle out along its own normal
    if (u.explode > 0.001) {
        p = p + n * u.explode * (0.25 + 0.95 * hash(a_rnd * 271.7));
    }

    // noise dissolves the head: a quarter of the particles stream away and
    // fade, each on its own clock, so the surface keeps shedding
    var diss = 0.0;
    if (u.dissipate > 0.001) {
        let pickd = hash(a_rnd * 613.7);
        if (pickd < 0.25 * u.dissipate) {
            let sp = 0.30 + 0.50 * hash(a_rnd * 77.7);
            let lifed = fract(hash(a_rnd * 131.1) + u.diss_t * sp);
            let away = normalize(n + vec3<f32>(hash(a_rnd * 19.3) - 0.5, -0.15, 0.55));
            p = p + away * lifed * lifed * 1.7 * u.dissipate;
            diss = lifed;
        }
    }

    // dust: crown and silhouette particles flake off and drift up
    let pick = hash(a_rnd * 917.3);
    let flaking = step(pick, u.dust * 0.035 * (0.2 + edge * 2.0 + 1.4 * w_high));
    let life = fract(a_rnd * 7.31 + u.time * (0.15 + 0.2 * hash(a_rnd * 31.0)));
    let drift = normalize(n + vec3<f32>(hash(a_rnd * 5.1) - 0.5, 0.0, 0.8));
    p = p + flaking * drift * life * 0.7;

    // The rings pass through the head as soft bands of light. They used to
    // light only the silhouette, through each particle's own normal --
    // which the scan's normal map makes noisy, so the band broke into a
    // jagged edge -- and to carry grains off the outline, which tore the
    // face. Now each is a wide, smooth band across the whole surface,
    // brighter toward the outline, and no particle moves.
    let rc = u.meter.xy;
    let rd = length(vec2<f32>(p.x, p.z) - rc);
    let sil = pow(1.0 - abs(dot(n, normalize(u.eye.xyz - p))), 2.0);
    var rims = array<vec4<f32>, 3>(u.rim0, u.rim1, u.rim2);
    var rim = 0.0;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let r = rims[i];
        if (r.y <= 0.001) { continue; }
        let x = (rd - r.x) / (r.z * 2.5);
        rim = rim + exp(-x * x) * r.y;
    }
    let carried = 0.0;

    var clip = u.view_proj * vec4<f32>(p, 1.0);

    // Exploding uncovers every particle the shell used to hide, so the fill
    // cost jumps. The cloud is dense enough to thin as it opens.
    var culled = false;
    if (u.explode > 0.35 && hash(a_rnd * 991.3) < 0.5 * (u.explode - 0.35)) {
        culled = true;
    }
    // The far side of the head is behind the shell, so roughly half the
    // cloud was drawn only to fail the depth test. Drop what faces away --
    // unless it is leaving the surface, where the shell no longer hides it.
    let to_eye = normalize(u.eye.xyz - p);
    let on_surface = u.explode < 0.02 && diss == 0.0 && flaking == 0.0 && carried == 0.0;
    if (on_surface && dot(n, to_eye) < -0.25) {
        culled = true;
    }

    // one hard key light from the camera's left and above, whichever side
    // the camera is on: the far side falls into real black, as in a
    // spot-lit photograph
    let key = normalize(u.key.xyz);
    let lam = pow(max(dot(n, key), 0.0), 1.35);
    // The beat's shockwave travels up the head as a band of *light*
    // rather than a bulge: it answers the music without moving a single
    // particle, so the face keeps its shape.
    let wd = length(part.pos - u.pivot);
    let crest = ripple(u.wave0, wd) + ripple(u.wave1, wd) + ripple(u.wave2, wd);

    var lum = (0.012 + 0.78 * lam) * part.fade * u.bright;
    lum = lum + crest * (0.35 + 0.9 * lam);

    // the rings' light: across the face, stronger toward the outline
    lum = lum + rim * (0.3 + 0.5 * sil) * part.fade;
    // Synth Cloud: a sheet of light sweeping the head with the LFO,
    // narrowing to a hard edge and shimmering as the filter resonates
    if (u.sheet.z > 0.001) {
        let shim = 0.75 + 0.25 * sin(u.sheet.w + a_rnd * 40.0);
        lum = lum + u.sheet.z * contour(zone, u.sheet.x, u.sheet.y) * shim * (0.25 + 0.95 * lam);
    }

    lum = lum * (0.55 + 0.75 * hash(a_rnd * 71.3));          // uneven grain
    lum = lum * (1.0 + 0.8 * u.low * w_low + 0.5 * u.mid * w_mid);
    // the song's shape: dim through a breakdown, full on the drop
    lum = lum * u.meter.z * (1.0 + u.meter.w);
    lum = lum * (1.0 + 1.6 * band_e);
    let tw = step(0.965, fract(a_rnd * 13.7 + u.time * 2.3));
    lum = lum + w_high * u.high * tw * lam * 1.2;            // highs glitter

    let col = vec3<f32>(lum) * mix(vec3<f32>(1.0), u.tint, u.tint_mix);
    var alpha = 1.0 - 0.45 * min(abs(streak) * 3.0, 1.0);
    alpha = alpha * (1.0 - flaking * life);
    alpha = alpha * (1.0 - diss);
    alpha = alpha * u.fade;
    if (carried > 0.0) {
        alpha = alpha * min(carried * 2.0, 1.0);
    }

    // mixed grain sizes, like a real particle photograph
    let size = 0.5 + 0.9 * pow(hash(a_rnd * 53.9), 3.0);
    // an exploding head throws particles at the camera, where perspective
    // makes every sprite bigger: shrink them as it opens, or the fill cost
    // triples and the frame rate falls through the floor
    let px = u.point * size * (1.0 - 0.45 * u.explode)
           * clamp(7.0 / max(clip.w, 0.001), 0.5, 2.5);

    var sp : Sprite;
    sp.clip = clip;
    // `px` is the sprite's diameter in pixels, as `gl_PointSize` was. A
    // radius of px/2 pixels spans px/vp of normalised device space, and
    // multiplying by clip.w carries that through the perspective divide.
    sp.radius = px * clip.w;
    // the fragment shader wrote rgb * a over an opaque 1, so pack that
    // alpha 0 marks the head for the effects after it (eye.w 0), else 1
    sp.color = pack4x8unorm(vec4<f32>(col * alpha, u.eye.w));
    if (culled) {
        sp.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        sp.radius = 0.0;
    }
    sprites_out[pi] = sp;
}

struct VsOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) @interpolate(flat) color : vec4<f32>,
    // the quad's own coordinate, for cutting the dot out
    @location(1) corner : vec2<f32>,
};

@vertex
fn vs_particles(@builtin(vertex_index) vid : u32) -> VsOut {
    let sp = sprites[vid / 6u];
    // expand to a quad: the six corners of two triangles
    var offs = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0,  1.0),
    );
    let corner = offs[vid % 6u];
    var clip = sp.clip;
    clip.x = clip.x + corner.x * sp.radius / u.vp_w;
    clip.y = clip.y + corner.y * sp.radius / u.vp_h;

    var out : VsOut;
    out.clip = clip;
    out.color = unpack4x8unorm(sp.color);
    out.corner = corner;
    return out;
}

@fragment
fn fs_particles(in : VsOut) -> @location(0) vec4<f32> {
    // solid round dots: a soft, blended edge would still write depth and
    // punch holes in the points behind it (a scaly, cracked look)
    if (dot(in.corner, in.corner) > 1.0) { discard; }
    return in.color;
}

// ─── the occluding shell ────────────────────────────────────────────────

struct ShellIn {
    @location(0) pos  : vec3<f32>,
    @location(1) zone : f32,
    @location(2) nrm  : vec3<f32>,
};

@vertex
fn vs_shell(v : ShellIn) -> @builtin(position) vec4<f32> {
    let d = deform(v.pos, v.nrm, v.zone);
    return u.view_proj * vec4<f32>(d.p, 1.0);
}

@fragment
fn fs_shell() -> @location(0) vec4<f32> {
    // the background colour: it exists only to fill the depth buffer --
    // and, like the particles, to mark the head in the alpha
    return vec4<f32>(0.02, 0.02, 0.04, u.eye.w);
}

// ─── the rings behind the head ──────────────────────────────────────────

struct RingIn {
    @location(0) pos   : vec3<f32>,
    @location(1) alpha : f32,
};

struct RingOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) alpha : f32,
};

@vertex
fn vs_rings(v : RingIn) -> RingOut {
    var out : RingOut;
    // the rings sit in the world, not on the head: the slicing and the
    // shake must not drag them about
    out.clip = u.view_proj * vec4<f32>(v.pos + u.shake * 0.25, 1.0);
    out.alpha = v.alpha;
    return out;
}

@fragment
fn fs_rings(in : RingOut) -> @location(0) vec4<f32> {
    // dark grey just above the background, a touch cool: they thicken the
    // air behind the subject rather than competing with it
    let grey = 0.19 + 0.07 * in.alpha;
    return vec4<f32>(grey, grey, grey * 1.06, 1.0) * in.alpha;
}
