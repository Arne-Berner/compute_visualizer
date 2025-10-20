// Vertex shader
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
  // Clamp to [0,1] to avoid invalid values
  let l = clamp(linear, vec3(0.0), vec3(1.0));
  let cutoff = vec3(0.0031308);
  let low = l * 12.92;
  let high = 1.055 * pow(l, vec3(1.0 / 2.4)) - 0.055;
  return select(low, high, l > cutoff);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
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
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let out = vec4f(linear_to_srgb(tex.xyz), 1.0);



    return out;
}
