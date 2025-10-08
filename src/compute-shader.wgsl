@group(0)
@binding(0)
var<storage, read_write> data: array<vec4<f32>>;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@compute
@workgroup_size(6, 6, 1)
fn odd_even_sort(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    let in: vec2<f32> = vec2f(f32(gid.x) / 256.0, f32(gid.y) / 256.0);

    let tex = textureSampleLevel(t_diffuse, s_diffuse, in, 0.0);
    // maybe get the image width via a uniform buffer
    data[gid.y * 256u + gid.x] = tex;
}
