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
    @location(2) normal: vec3<f32>
}

struct InstanceInput {
    @location(12) position: vec4<f32>,
    @location(13) scale: vec4<f32>,
    @location(14) color: vec4<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var particle_origin = instance.position.xyz;
    var camera_to_particle = normalize(model_uniform.camera_pos.xyz - particle_origin);

    var cos_theta = cos(instance.position.w);
    var sin_theta = sin(instance.position.w);
    var vertex_pos: vec3<f32>;
    vertex_pos.x = model.position.x * cos_theta - model.position.y * sin_theta;
    vertex_pos.y = model.position.x * sin_theta + model.position.y * cos_theta;
    vertex_pos.z = 0.0;

    var right_vec = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), camera_to_particle));
    var up_vec = cross(camera_to_particle, right_vec);

    right_vec = right_vec * vertex_pos.x * instance.scale.x;
    up_vec = up_vec * vertex_pos.y * instance.scale.x;  // Todo: uniform scale only
    var pos = particle_origin + up_vec + right_vec;

    var out: VertexOutput;
    out.clip_position = model_uniform.world_view_proj * vec4<f32>(pos.xyz, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = instance.color;
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
    var albedo: vec4<f32> = textureSample(t_diffuse, s_diffuse, uv);
    outColor = albedo * in.color;
    outColor.r = pow(outColor.r, model_uniform.time_colorpow_.y);
    outColor.g = pow(outColor.g, model_uniform.time_colorpow_.y);
    outColor.b = pow(outColor.b, model_uniform.time_colorpow_.y);
    return outColor;
}