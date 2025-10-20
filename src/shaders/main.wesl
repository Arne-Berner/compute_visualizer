@group(0) @binding(0)
var in_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1)
var out_tex: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(6, 6, 1)
fn odd_even_sort(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    let norm_gid = vec3f(f32(gid.x)/256.0, f32(gid.y)/256.0, f32(gid.z)/256.0);
    let in: vec2<f32> = vec2f(norm_gid.x, norm_gid.y);
    var tex = textureLoad(in_tex, gid.xy);
    // var tex = textureSampleLevel(t_diffuse, s_diffuse, in, 0.0); // same thing?

    // tex = tex - 0.5;

    // let color: vec4<f32> = vec4f(norm_gid.xyz, 1.0);
    var color = vec4f(0.5,0.5,0.5,1.0);
    if tex.r == 1.0 {
        color = tex;
    }

    // maybe get the image width via a uniform buffer
    textureStore(out_tex, vec2<i32>(gid.xy), color);
}
