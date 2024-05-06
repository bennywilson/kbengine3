struct ModelUniform {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
    world_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    target_dimensions: vec4<f32>,
    time_colorpow_: vec4<f32>
};
@group(1) @binding(0)
var<uniform> model_uniform: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) inv_light_1: vec3<f32>,
    @location(3) inv_light_2: vec3<f32>,
    @location(4) inv_light_3: vec3<f32>,
    @location(5) cam_to_vert: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = model.tex_coords;

    var pos: vec3<f32> = model.position.xyz * 0.3;
    var normal = vec4<f32>(model.normal.xyz, 0.0);
    var normalized = normalize(normal.xyz);
    out.normal = (model_uniform.world * normal).xyz;
    
    var world_pos: vec4<f32> = model_uniform.world * vec4<f32>(pos.xyz, 1.0);
    out.cam_to_vert = normalize(world_pos.xyz - model_uniform.camera_pos.xyz);
    world_pos.x += out.cam_to_vert.x * 0.5;
    world_pos.y += out.cam_to_vert.y * 0.5;
    world_pos.z += out.cam_to_vert.z * 0.5;

    out.clip_position = model_uniform.view_proj * vec4<f32>(world_pos.xyz, 1.0);
    out.inv_light_1 = (model_uniform.inv_world * vec4<f32>(1.0, 1.0, 1.0, 0.0)).xyz;
    out.inv_light_2 = (model_uniform.inv_world * vec4<f32>(-1.0, 1.0, 1.0, 0.0)).xyz;
    out.inv_light_3 = (model_uniform.inv_world * vec4<f32>(0.0, 1.0, 0.0, 0.0)).xyz;
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
    uv = in.tex_coords;
    var albedo: vec3<f32> = textureSample(t_diffuse, s_diffuse, uv).xyz;

    var normal = normalize(in.normal);
    var dot_prod: f32 = saturate(dot(normal, normalize(in.inv_light_1)));
    var light_1 = dot_prod * vec3<f32>(1.0, 1.0, 1.0) * 0.5;

    dot_prod = saturate(dot(normal, normalize(in.inv_light_2)));
    var light_2 = dot_prod * vec3<f32>(1.0, 1.0, 1.0) * 0.5;

    dot_prod = saturate(dot(normal, normalize(in.inv_light_3)));
    var light_3 = dot_prod * vec3<f32>(0.0, 0.0, 0.0);

    light_1 = light_1 * 0.9 + 0.1;
    light_2 = light_2 * 0.9 + 0.1;
    light_3 = light_3 * 0.9 + 0.1;

    var lighting: vec3<f32> = albedo * light_1 + albedo * light_2 + albedo * light_3;

    outColor.x = lighting.x;
    outColor.y = lighting.y;
    outColor.z = lighting.z;
    outColor.w = 1.0;

    var val = 1.0 - (0.5 +  0.5 * pow(1.0 + dot(normal.xyz, -normalize(in.cam_to_vert.xyz)), 1.0));
    outColor.r = val;//pow(outColor.r, model_uniform.time_colorpow_.y);
    outColor.g = val;//pow(outColor.g, model_uniform.time_colorpow_.y);
    outColor.b = val;//pow(outColor.b, model_uniform.time_colorpow_.y);
    outColor.a = 1.0;//
    return outColor;
}