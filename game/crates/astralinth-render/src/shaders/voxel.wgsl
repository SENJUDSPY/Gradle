struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    sky_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) face_lighting: f32,
    @location(4) texture_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) face_lighting: f32,
    @location(2) @interpolate(flat) texture_index: u32,
    @location(3) world_pos: vec3<f32>,
    @location(4) normal: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = model.position;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.face_lighting = model.face_lighting;
    out.texture_index = model.texture_index;
    out.normal = model.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords, in.texture_index);

    // Alpha cutout for transparent leaves
    if (tex_color.a < 0.1) {
        discard;
    }

    // Dynamic celestial sunlight calculation
    let sun_dot = max(dot(in.normal, camera.sun_dir.xyz), 0.0);
    let sun_brightness = camera.sun_dir.w; // 0.08 to 1.0
    let diffuse_sun = sun_dot * sun_brightness * 0.45;
    let ambient = max(0.12, sun_brightness * 0.55);

    let total_light = (diffuse_sun + ambient) * in.face_lighting;
    let lit_rgb = tex_color.rgb * total_light;

    // Atmospheric fog blending into the dynamic sky color
    let dist = distance(in.world_pos, camera.camera_pos.xyz);
    let fog_start = 50.0;
    let fog_end = 220.0;
    let fog_factor = clamp((dist - fog_start) / (fog_end - fog_start), 0.0, 1.0);

    let final_rgb = mix(lit_rgb, camera.sky_color.rgb, fog_factor);
    return vec4<f32>(final_rgb, tex_color.a);
}
