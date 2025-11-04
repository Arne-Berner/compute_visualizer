fn s_min(a: f32, b: f32, tolerance: f32 ) -> f32
{
    
    // k *= 1.0; // different values for different effects
    let r = exp2(-a/tolerance) + exp2(-b/tolerance);
    return -tolerance*log2(r);
}

fn ud_segment(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let ba = b-a;
    let pa = p-a;
    // in his video this is not h, as drawn
    // h in his drawing is h*ba
    let h = clamp(dot(pa,ba)/dot(ba,ba), 0.0, 1.0);

    // so here it substracts the projection of P-A on the line from the P-A
    // leaving the distance to the line
    return length(pa-h*ba);
}

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       tex_coords: vec2<f32>,
};


@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    // 3-vertex fullscreen triangle
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    // Matching UVs for the fullscreen triangle
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0)
    );

    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    out.tex_coords = uv[vid];
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // vertex position.x / dimension.x und vertex position.y / dimension.y 

    let p = 2.0 * in.tex_coords.xy - 1.0;
    let start = vec2f(0.0, 0.5);
    let end = vec2f(0.0, -0.5);
    let start2 = vec2f(0.5, -0.5);
    let end2 = vec2f(-0.5, 0.5);
    let d = ud_segment(p, start, end);
    let d2 = ud_segment(p, start2, end2);
    let out = s_min(d, d2, 0.01);

    // out starts with 0 on a line and will rise further away
    // so w will be the point, when out is allowed to rise via smoothstep
    let w = 0.00015;
    // is the partial derivative of the current texel and the neighbouring ones in a 2x2 grid
    let aa = fwidth(out);

    // so w and w+aa will be the strip between the current and the next pixel
    // this strip is determined by the zoom... I think
    var col = vec3(1.0) - smoothstep(w, w+aa, out);

    let fg = vec3(1.0);
    let bg = vec3(0.9,0.6,0.3);

    col = mix(bg, fg, col); 


    return vec4f(col, 1.0);
}
