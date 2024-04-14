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
    @location(2) instance_data: f32,
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
    out.instance_data = instance.instance_data[0];
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
    var fractional_part = in.instance_data - floor(in.instance_data);
    if (fractional_part < 0.1) {
        uvs.y = 1.0 - uvs.y;
    } else if (fractional_part < 0.3) {
        uvs.x = 1.0 - uvs.x;
    } else if (fractional_part < 0.6 ) {
        uvs.x = 1.0 - uvs.x;
        uvs.y = 1.0 - uvs.y;
    }

    var atlas_uvs: vec2<f32> = in.atlas_uvs;
    var time_scale = in.instance_data + modelBuffer.time.x;
    var noise1_uvs: vec2<f32> = (uvs * 0.3) + (vec2<f32>(1.0, 0.4) * time_scale * 0.03);
    var noise1_color: f32 = smoothstep(0.0, 0.7, textureSample(t_noise, s_diffuse, noise1_uvs).g);

    var noise2_uvs: vec2<f32> = (uvs * 0.5) + (vec2<f32>(-0.3, 0.7) * time_scale * 0.04);
    var noise2_color: f32 = smoothstep(0.0, 0.7, textureSample(t_noise, s_diffuse, noise2_uvs).g);

    var noise3_uvs: vec2<f32> = (uvs * 0.7) + (vec2<f32>(0.1, 0.4) * time_scale * 0.02);
    var noise3_color: f32 = smoothstep(0.0, 0.7, textureSample(t_noise, s_diffuse, noise3_uvs).g);

    var noise4_uvs: vec2<f32> = (uvs * 0.2) + (vec2<f32>(1.0, -0.1) * time_scale * 0.01);
    var noise4_color: f32 = smoothstep(0.0, 0.7, textureSample(t_noise, s_diffuse, noise4_uvs).g);

    var noise_color =  (noise1_color + noise2_color + noise3_color + noise4_color) * 0.5;
  //  noise_color = smoothstep(0.1, 0.8, noise_color);
    outColor = textureSample(t_diffuse, s_diffuse, atlas_uvs);
    var cloud_color: f32 = noise_color;
    outColor.a = cloud_color;
    outColor.r *= cloud_color;
    outColor.g *= cloud_color;
    outColor.b *= cloud_color;
    outColor = outColor * 0.9 + 0.1 ;

	var edge_alpha_x: f32 = min( smoothstep( 0.0, 0.25, uvs.x ), 1.0 - smoothstep( 0.75, 1.0, uvs.x ) );
	var edge_alpha_y: f32 = min( smoothstep( 0.0, 0.25, uvs.y ), 1.0 - smoothstep( 0.75, 1.0, uvs.y ) );
    var edge_alpha: f32 = min( edge_alpha_x, edge_alpha_y );
    var cloud_alpha: f32 = textureSample(t_noise, s_diffuse, uvs).b * 1.61 * smoothstep(0.0, 0.6, noise_color);
    outColor.a = cloud_alpha - (outColor.r * 0.6);//smoothstep(0.0, 1.0, cloud_alpha);

/*    outColor.r = pow(outColor.r, 1.0 / 2.2);
    outColor.g = pow(outColor.g, 1.0 / 2.2);
    outColor.b = pow(outColor.b, 1.0 / 2.2);*/2
    return outColor;
}