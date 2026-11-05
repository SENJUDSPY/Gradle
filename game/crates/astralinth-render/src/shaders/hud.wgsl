@group(0) @binding(0)
var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) texture_index: i32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) texture_index: i32,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    out.texture_index = model.texture_index;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.texture_index >= 0) {
        let tex_col = textureSample(t_diffuse, s_diffuse, in.tex_coords, u32(in.texture_index));
        return tex_col * in.color;
    }
    return in.color;
}
