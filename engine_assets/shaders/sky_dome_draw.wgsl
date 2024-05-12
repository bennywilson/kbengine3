struct ModelUniform {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
    world_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    target_dimensions: vec4<f32>,
    time_colorpow_ : vec4<f32>,
    model_color: vec4<f32>,
    custom_data_1: vec4<f32>,
    sun_color: vec4<f32>
};

@group(1) @binding(0)
var<uniform> model_uniform: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) inv_light_1: vec3<f32>,
    @location(3) inv_light_2: vec3<f32>,
    @location(4) inv_light_3: vec3<f32>
}

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = model.tex_coords;

    var pos: vec3<f32> = model.position.xyz * 0.3;
    var normal = vec4<f32>(model.normal.xyz, 0.0);
    out.normal = (model_uniform.inv_world * normal).xyz;

    out.clip_position = model_uniform.world_view_proj * vec4<f32>(pos.xyz, 1.0);
    out.inv_light_1 = (model_uniform.inv_world * vec4<f32>(1.0, 1.0, 1.0, 0.0)).xyz;
    out.inv_light_2 = (model_uniform.inv_world * vec4<f32>(-1.0, 1.0, 1.0, 0.0)).xyz;
    out.inv_light_3 = (model_uniform.inv_world * vec4<f32>(0.0, 1.0, 0.0, 0.0)).xyz;

//out.clip_position.z = 0.5;
//out.clip_position.w = 0.5;

    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_noise: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var outColor: vec4<f32>; 

    // Sample 1
    var sample_1_scroll = 0.005f;
    var sample_1_uvs = in.tex_coords * vec2<f32>(1.02, 0.5) + vec2<f32>(0.0, 0.0);
    sample_1_uvs += vec2<f32>(model_uniform.time_colorpow_.x * sample_1_scroll, 0.0);

    var mask_1 = textureSample(t_diffuse, s_diffuse, sample_1_uvs);
    mask_1.r = mask_1.r * 0.5 + 0.5;

    var out_color_1 = mask_1.rrrg;
    out_color_1.x *= model_uniform.sun_color.x * 2;
    out_color_1.y *= model_uniform.sun_color.y * 2;
    out_color_1.z *= model_uniform.sun_color.z * 2;

    // Sample 2
    var sample_2_scroll = 0.008;
    var sample_2_uvs = in.tex_coords * vec2<f32>(0.5, 0.25) + vec2<f32>(0.0, 0.0);
    sample_2_uvs += vec2<f32>(model_uniform.time_colorpow_.x * sample_2_scroll, 0.0);

    var mask_2 = textureSample(t_diffuse, s_diffuse, sample_2_uvs);
    mask_2.r = mask_2.r * (model_uniform.sun_color.w) + (1.0 - model_uniform.sun_color.w);

    var out_color_2 = mask_2.rrrg;
    out_color_2.x *= model_uniform.sun_color.x;
    out_color_2.y *= model_uniform.sun_color.y;
    out_color_2.z *= model_uniform.sun_color.z;

    // Composite
    var out_color = mix(out_color_1, out_color_2, 0.4);
    out_color.a = smoothstep(0.1, 0.4,  out_color.a);
	out_color.a *= 1.0 * in.tex_coords.y;

    return out_color;
}