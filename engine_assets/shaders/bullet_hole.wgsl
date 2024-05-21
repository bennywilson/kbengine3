struct ModelUniform {
    trace_hit: vec4f,
    trace_dir: vec4f,
};

@group(0) @binding(0)
var<uniform> model_uniform: ModelUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) normal: vec3f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) local_pos: vec3f,
}

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    var pos: vec2f = model.tex_coords.xy * 2.0 - 1.0;
    pos.y *= -1.0;
    out.clip_position = vec4f(pos.xy, 0.0, 1.0);
    out.local_pos = model.position;
    var swap =  out.local_pos.z;
    out.local_pos.z *= -1.0;
    return out;
}


// Fragment shader
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var trace_hit = model_uniform.trace_hit.xyz;
    var trace_dir = model_uniform.trace_dir.xyz;
	var closestPt = dot(in.local_pos.xyz - trace_hit, trace_dir) * trace_dir + trace_hit;
    var hole_size = 75.0f;
    var scorch_size = 3.0;

    var out_color: vec4f = vec4f(1.0, 1.0, 1.0, 0.0);

    var local_pos = in.local_pos.xyz;
	if ( length( closestPt - local_pos ) > 0.7f ) {
		out_color.w = 1.0;
    }

	var normalized_dist = saturate( length( closestPt - local_pos.xyz ) / hole_size );
	if ( normalized_dist > 0.4f ) {
	    out_color.w = 1.0;
	}

    // Hack.  Only works on planes on yz    
	var scorch_uv = (local_pos.yz - closestPt.yz) / scorch_size;
	scorch_uv = scorch_uv * 0.5 + 0.5;
    scorch_uv = saturate(scorch_uv);

    var scorch = textureSample(t_diffuse, s_diffuse, scorch_uv).xyz;
	out_color.x *= scorch.x;
	out_color.y *= scorch.y;
	out_color.z *= scorch.z;


	return out_color;
}