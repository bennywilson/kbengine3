struct Uniform {
    world_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    target_dimensions: vec4<f32>,
    uv_scale_offset: vec4<f32>,
    extra_data: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> uniform: Uniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(10) pos_scale: vec4<f32>,

}
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in_vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in_vertex.position.xyz, 1.0);
    out.clip_position.x *= instance.pos_scale.z;
    out.clip_position.y *= instance.pos_scale.z;

    out.tex_coords = in_vertex.tex_coords;
    return out;
}

/**
 *  Fragment Shader
 */

@group(1) @binding(0)
var mask_texture: texture_2d<f32>;
@group(1) @binding(1)
var mask_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var out = vec4<f32>(0.3, 0.3, 0.0, 0.0) * textureSample(mask_texture, mask_sampler, in.tex_coords.xy).xyzw;
    return out;
}