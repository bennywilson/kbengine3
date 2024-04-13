// Vertex shader
struct ModelUniform {
    time: vec4<f32>
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
    @location(4) instance_data: vec4<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
    @location(1) atlas_uvs: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.atlas_uvs = (model.tex_coords * instance.uv_scale_bias.xy) + instance.uv_scale_bias.zw;
    out.uvs = model.tex_coords;

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
    var uvs: vec2<f32> = in.uvs;
    var atlas_uvs: vec2<f32> = in.atlas_uvs;

    var noise1_uvs: vec2<f32> = (uvs * 0.7) + (vec2<f32>(1.0, 0.4) * modelBuffer.time.x * 0.1);
    var noiseColor_1: vec4<f32> = textureSample(t_noise, s_diffuse, noise1_uvs).rgba;

    var noise2_uvs: vec2<f32> = (uvs * 0.7) + (vec2<f32>(1.0, -0.2) * modelBuffer.time.x * 0.05);
    var noiseColor_2: vec4<f32> = textureSample(t_noise, s_diffuse, noise2_uvs).rgba;

    var noise_color = smoothstep(0.3, 0.8, ((noiseColor_1 + noiseColor_2) * 0.5).g);

    outColor = textureSample(t_diffuse, s_diffuse, atlas_uvs);
    var cloud_color: f32 = (noise_color * 0.8) + 0.2;//smoothstep(0.0, 1.0, (noise_color.g * 0.8) + 0.2);
    outColor.r *= cloud_color;
    outColor.g *= cloud_color;
    outColor.b *= cloud_color;

    var cloud_alpha: f32 = textureSample(t_noise, s_diffuse, uvs).b * (textureSample(t_noise, s_diffuse, uvs).b + noise_color);
    outColor.a = smoothstep(0.0, 1.0, cloud_alpha);
    return outColor;
}