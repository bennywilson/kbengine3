struct ModelUniform {
    world_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    flare_position_scale: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> uniform_buffer: ModelUniform;

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
    var flare_pos_scale = uniform_buffer.flare_position_scale;
    var particle_origin = uniform_buffer.camera_pos.xyz + (flare_pos_scale.xyz * 3.0);

    var camera_to_particle = -uniform_buffer.camera_dir.xyz;
    var vertex_pos = vec3<f32>(in_vertex.position.x, in_vertex.position.y, 0.0);

    var right_vec = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), camera_to_particle));
    var up_vec = cross(camera_to_particle, right_vec);

    right_vec = right_vec * vertex_pos.x * flare_pos_scale.w;
    up_vec = up_vec * vertex_pos.y * flare_pos_scale.w;
    var pos = particle_origin + up_vec + right_vec;

    var out: VertexOutput;
    out.clip_position = uniform_buffer.world_view_proj * vec4<f32>(pos.xyz, 1.0);
    out.clip_position.x /= out.clip_position.w;
    out.clip_position.y /= out.clip_position.w;
    out.clip_position.z = 0.99999;
    out.clip_position.w = 1.0;

    out.tex_coords = in_vertex.tex_coords;
    return out;
}

/**
 *  Fragment Shader
 */

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.tex_coords.xy, 1.0, 1.0);
}