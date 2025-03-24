struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct Fragment {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
};

@group(0)
@binding(0)
var tex_sampler: sampler; // Fragment
@group(1)
@binding(0)
var<uniform> state_matrix: mat4x2<f32>; // Vertex
@group(2)
@binding(0)
var tex: texture_2d<f32>; // Fragment

@vertex
fn vs_main(@builtin(vertex_index) index: u32, vertex: Vertex) -> Fragment {
    var frag: Fragment;
    frag.clip_position = vec4f(state_matrix * vec4f(vertex.pos, 0.0, 1.0), 0.0, 1.0);
    frag.uv = vertex.uv;
    frag.color = vertex.color;
    return frag;
}

@fragment
fn fs_main(frag: Fragment) -> @location(0) vec4f {
    return textureSample(tex, tex_sampler, frag.uv) * frag.color;
}
