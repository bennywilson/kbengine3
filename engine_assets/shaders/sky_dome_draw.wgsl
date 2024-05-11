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
    var uv : vec2<f32>; 

    // Sample 1
    var sample_1_scroll = 0.005f;
    uv = in.tex_coords * vec2<f32>(1.02, 0.5) + vec2<f32>(0.0, 0.0);
    uv = uv + vec2<f32>(model_uniform.time_colorpow_.x * sample_1_scroll, 0.0);
    var mask_1 = textureSample(t_diffuse, s_diffuse, uv);
    mask_1.r = mask_1.r * 0.5 + 0.5;
    var out_color_1 = mask_1.rrrg * vec4<f32>(0.8 * 2.0, 0.58 * 2.0, 0.24 * 2.0, 1.0);

    // Sample 2
    var sample_2_scroll = 0.008;
    uv = in.tex_coords * vec2<f32>(0.5, 0.25) + vec2<f32>(0.0, 0.0);
    uv = uv + vec2<f32>(model_uniform.time_colorpow_.x * sample_2_scroll, 0.0);
    var mask_2 = textureSample(t_diffuse, s_diffuse, uv);
    mask_2.r = mask_2.r * 0.8 + 0.2;
    var out_color_2 = mask_2.rrrg * vec4<f32>(0.8, 0.58, 0.24, 1.0);
    var out_color = mix(out_color_1, out_color_2, 0.4);

    out_color.a = smoothstep(0.1, 0.4,  out_color.a);


	var edgeFade = 1.0 - saturate((in.tex_coords.y - 0.9) / 0.9);
	out_color.a *= edgeFade * out_color.a;

    return out_color;
}