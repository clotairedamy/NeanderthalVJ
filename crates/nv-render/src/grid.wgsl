// The Hiding Grid: a flat wall of squares and its lattice.
//
// Everything about how it looks -- the waves, the lean, the colours -- is
// worked out per vertex on the CPU (see grid.rs), so this only fits the
// [-1, 1] field to the viewport without stretching it and applies the
// palette's tint.

struct GridUniforms {
    // scales the [-1, 1] field to the viewport without stretching it
    fit   : vec2<f32>,
    tint  : vec3<f32>,
    mix   : f32,
};

@group(0) @binding(0) var<uniform> g : GridUniforms;

struct In {
    @location(0) pos   : vec2<f32>,
    @location(1) color : vec3<f32>,
};

struct Out {
    @builtin(position) clip : vec4<f32>,
    @location(0) color : vec3<f32>,
};

@vertex
fn vs_grid(v : In) -> Out {
    var out : Out;
    out.clip = vec4<f32>(v.pos * g.fit, 0.0, 1.0);
    out.color = v.color;
    return out;
}

@fragment
fn fs_grid(in : Out) -> @location(0) vec4<f32> {
    return vec4<f32>(mix(in.color, in.color * g.tint, g.mix), 1.0);
}
