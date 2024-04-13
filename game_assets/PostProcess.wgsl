// Vertex shader
struct PostProcessUniform {
    time: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> postprocess_buffer: PostProcessUniform;

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

) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = model.tex_coords ;//(model.tex_coords * instance.uv_scale_bias.xy) + instance.uv_scale_bias.zw;

    var pos: vec3<f32> = model.position.xyz;
  //  pos *= vec3<f32>(instance.pos_scale.zw, 1.0);
  //  pos += vec3<f32>(instance.pos_scale.xy, 1.0);
    out.clip_position = vec4<f32>(pos.xyz, 1.0);

    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_post_process_filter: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_scene_color: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    var outColor: vec4<f32>;
    var uv : vec2<f32>; 
    uv = in.tex_coords;

    // Luminance
    outColor = textureSample(t_scene_color, s_diffuse, uv);
    outColor.x = dot(outColor.xyz, vec3<f32>(0.3,0.59,0.11));
    outColor.y = outColor.x;
    outColor.z = outColor.x;

    // Scan line
    var uv_offset: vec2<f32> = vec2<f32>(0.0, postprocess_buffer.time.x * -0.02f);
    var uv_scale: vec2<f32> = vec2<f32>(0.5, 0.5);
    var scanLine: vec4<f32> = textureSample(t_post_process_filter, s_diffuse, uv * uv_scale + uv_offset);
    outColor = textureSample(t_scene_color, s_diffuse, uv);
    outColor = outColor * ((scanLine * 0.5) + 0.5); 

   // var to_center: vec2<f32> = uv - vec2<f32>(0.5, 0.5);
   //0 var perp: vec2<f32> = normalize(vec2<f32>(to_center.x, to_center.y));
    return outColor;

}