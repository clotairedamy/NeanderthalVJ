// Post-processing: the FX tab, applied to whatever the stage drew.
//
// `fs_bright` keeps what is brighter than the threshold; the two blurs
// spread it into a glow; `fs_composite` puts the frame back together with
// the glow, the beat's flash, a colour split on kicks, film grain on the
// drums, a vignette and a fading echo of the frames before -- and writes
// the result twice: to the screen, and into the history the next frame's
// echo is read from.

struct Post {
    // where the stage sits in the target, in 0..1: x, y, width, height
    rect     : vec4<f32>,
    bloom    : f32,
    threshold: f32,
    flash    : f32,
    grain    : f32,
    chroma   : f32,
    vignette : f32,
    ghost    : f32,
    time     : f32,
    aspect   : f32,
    pad0     : f32,
    pad1     : f32,
    pad2     : f32,
};

@group(0) @binding(0) var t0 : texture_2d<f32>;
@group(0) @binding(1) var t1 : texture_2d<f32>;
@group(0) @binding(2) var t2 : texture_2d<f32>;
@group(0) @binding(3) var smp : sampler;
@group(0) @binding(4) var<uniform> post : Post;

struct Full {
    @builtin(position) pos : vec4<f32>,
    @location(0) uv : vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) i : u32) -> Full {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o : Full;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

@fragment
fn fs_bright(in : Full) -> @location(0) vec4<f32> {
    let c = textureSample(t0, smp, in.uv).rgb;
    let l = max(c.r, max(c.g, c.b));
    // a soft knee, so the glow fades in rather than switching on
    let k = smoothstep(post.threshold, post.threshold + 0.25, l);
    return vec4<f32>(c * k, 1.0);
}

fn blur(uv : vec2<f32>, dir : vec2<f32>) -> vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(t0));
    // nine taps, spaced wide: a big soft glow from few samples
    var w = array<f32, 5>(0.227, 0.194, 0.121, 0.054, 0.016);
    var c = textureSample(t0, smp, uv).rgb * w[0];
    for (var i = 1; i < 5; i = i + 1) {
        let o = dir * texel * f32(i) * 1.8;
        c = c + textureSample(t0, smp, uv + o).rgb * w[i];
        c = c + textureSample(t0, smp, uv - o).rgb * w[i];
    }
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_blur_h(in : Full) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2<f32>(1.0, 0.0));
}

@fragment
fn fs_blur_v(in : Full) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2<f32>(0.0, 1.0));
}

fn hash(p : vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

struct Out {
    @location(0) screen  : vec4<f32>,
    @location(1) history : vec4<f32>,
};

@fragment
fn fs_composite(in : Full) -> Out {
    // `in.uv` runs across the stage; the textures cover the whole target
    let suv = post.rect.xy + in.uv * post.rect.zw;
    let d = in.uv - vec2<f32>(0.5);

    // the colour split: red and blue pulled apart along the radius
    let off = d * post.chroma * 0.012 * post.rect.zw;
    var col = vec3<f32>(
        textureSample(t0, smp, suv + off).r,
        textureSample(t0, smp, suv).g,
        textureSample(t0, smp, suv - off).b,
    );

    col = col + textureSample(t1, smp, suv).rgb * post.bloom;

    // the echo: the last frames, a little larger, fading
    let guv = post.rect.xy + (vec2<f32>(0.5) + d * 0.965) * post.rect.zw;
    col = col + textureSample(t2, smp, guv).rgb * post.ghost;

    col = col * (1.0 + post.flash);

    var dv = d;
    dv.x = dv.x * post.aspect;
    let v = 1.0 - smoothstep(0.32, 0.92, length(dv));
    col = col * mix(1.0, v, post.vignette);

    let n = hash(in.uv * vec2<f32>(1931.0, 1093.0) + vec2<f32>(post.time * 37.0, post.time * 11.0));
    col = max(col + (n - 0.5) * post.grain, vec3<f32>(0.0));

    var o : Out;
    o.screen = vec4<f32>(col, 1.0);
    o.history = vec4<f32>(col, 1.0);
    return o;
}
