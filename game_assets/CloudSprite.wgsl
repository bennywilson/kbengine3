// Vertex shader
struct ModelUniform {
    worldPosition: vec4<f32>,
    uvOffset: vec4<f32>
};
@group(1) @binding(0)
var<uniform> modelBuffer: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(2) pos_scale: vec4<f32>,
    @location(3) uv_scale_bias: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = (model.tex_coords * instance.uv_scale_bias.xy) + instance.uv_scale_bias.zw;

    var pos: vec3<f32> = model.position.xyz;
    pos *= vec3<f32>(instance.pos_scale.zw, 1.0);
    pos += vec3<f32>(instance.pos_scale.xy, 1.0);
    out.clip_position = vec4<f32>(pos.xyz, 1.0);

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
6
    var noiseColor_1: vec4<f32> = textureSample(t_noise, s_diffuse, uv * 5.0).bbbb;

    outColor = textureSample(t_diffuse, s_diffuse, uv);
    outColor.r *= noiseColor_1.r;
    outColor.g *= noiseColor_1.r;
    outColor.b *= noiseColor_1.r;

    if (outColor.a < 0.5) {
        discard;
    }

    return outColor;
}