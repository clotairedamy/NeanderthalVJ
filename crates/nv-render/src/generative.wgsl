// Generative Geometry, on the GPU.
//
// Three stages. `fs_feedback` carries the last frame into this one,
// zoomed, turned, faded and slightly hue-shifted, which is what grows the
// lines into structures. `fs_shape` adds this frame's lines on top with a
// soft edge, additively. `fs_composite` tone-maps the result onto the
// stage.

struct Feedback {
    zoom: f32,
    rotate: f32,
    decay: f32,
    aspect: f32,
    exposure: f32,
    hue: f32,
    pad0: f32,
    pad1: f32,
};

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var<uniform> fb: Feedback;

struct Full {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// one triangle that covers the viewport
@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> Full {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: Full;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

@fragment
fn fs_feedback(in: Full) -> @location(0) vec4<f32> {
    // about the centre, in square units, so the turn is not sheared
    var d = in.uv - vec2<f32>(0.5);
    d.x = d.x * fb.aspect;
    let c = cos(fb.rotate);
    let s = sin(fb.rotate);
    d = vec2<f32>(c * d.x - s * d.y, s * d.x + c * d.y) / fb.zoom;
    d.x = d.x / fb.aspect;
    let uv = d + vec2<f32>(0.5);
    // nothing comes in from outside the frame
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    let col = textureSample(source, smp, uv).rgb;

    // a slow turn of the hue around the grey axis, so old structure
    // drifts away from the colours of what is playing now
    let k = vec3<f32>(0.57735027);
    let ch = cos(fb.hue);
    let sh = sin(fb.hue);
    let turned = col * ch + cross(k, col) * sh + k * dot(k, col) * (1.0 - ch);
    return vec4<f32>(max(turned, vec3<f32>(0.0)) * fb.decay * select(0.0, 1.0, inside), 1.0);
}

struct ShapeIn {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) edge: f32,
};

struct ShapeOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) edge: f32,
};

@vertex
fn vs_shape(v: ShapeIn) -> ShapeOut {
    var o: ShapeOut;
    o.pos = vec4<f32>(v.pos, 0.0, 1.0);
    o.color = v.color;
    o.edge = v.edge;
    return o;
}

@fragment
fn fs_shape(v: ShapeOut) -> @location(0) vec4<f32> {
    // brightest along the middle of the line, nothing at its edges
    let a = pow(1.0 - clamp(abs(v.edge), 0.0, 1.0), 1.6) * v.color.a;
    return vec4<f32>(v.color.rgb * a, a);
}

@fragment
fn fs_composite(in: Full) -> @location(0) vec4<f32> {
    let c = textureSample(source, smp, in.uv).rgb;
    // the trails add up without bound; this keeps them luminous, not white
    let mapped = vec3<f32>(1.0) - exp(-c * fb.exposure);
    var d = in.uv - vec2<f32>(0.5);
    d.x = d.x * fb.aspect;
    let vignette = 1.0 - smoothstep(0.42, 0.95, length(d));
    return vec4<f32>(mapped * (0.4 + 0.6 * vignette) + vec3<f32>(0.006, 0.006, 0.014), 1.0);
}
