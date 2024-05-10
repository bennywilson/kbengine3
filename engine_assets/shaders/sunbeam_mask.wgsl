struct Uniform {
    world_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    target_dimensions: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> uniform: Uniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in_vertex: VertexInput) -> VertexOutput {
    var particle_origin = uniform.camera_pos.xyz + vec3<f32>(500.0, 650.0, 500.0);
    var particle_scale = 150.0;

    var camera_to_particle = -uniform.camera_dir.xyz;
    var vertex_pos = vec3<f32>(in_vertex.position.x, in_vertex.position.y, 0.0);

    var right_vec = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), camera_to_particle));
    var up_vec = cross(camera_to_particle, right_vec);

    right_vec = right_vec * vertex_pos.x * particle_scale;
    up_vec = up_vec * vertex_pos.y * particle_scale;
    var pos = particle_origin + up_vec + right_vec;

    var out: VertexOutput;
    out.clip_position = uniform.world_view_proj * vec4<f32>(pos.xyz, 1.0);
    out.tex_coords = in_vertex.tex_coords;
    return out;
}

/**
 *  Fragment Shader
 */

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}