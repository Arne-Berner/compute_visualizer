// Vertex/fragment WGSL for instanced quad + SDF line rendering.
// - Vertex inputs:
//   @location(0) corner: vec2<f32>  (x along segment in [-1,1], y offset in [-1,1])
//   @location(1) a: vec2<f32>       endpoint A in pixel coordinates
//   @location(2) b: vec2<f32>       endpoint B in pixel coordinates
//   @location(3) radius: f32        radius in pixels
// - Uniform: screen size in pixels (width, height)
// - Fragment: compute distance from fragment pixel to segment AB in pixel space, output white when inside radius.

struct VertexIn {
    @location(0) corner: vec2<f32>,
    @location(1) a: vec2<f32>,
    @location(2) b: vec2<f32>,
    @location(3) radius: f32,
}


struct VSOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) frag_pos_px: vec2<f32>, // fragment position in pixels (interpolated)
    @location(1) a_px: vec2<f32>,
    @location(2) b_px: vec2<f32>,
    @location(3) radius: f32,
};

@group(0) @binding(0) var<uniform> u_screen: vec2<f32>; // width,height

@vertex
fn vs_main(
    in: VertexIn,
) -> VSOut {
    var out: VSOut;

    // t along segment: map corner.x from [-1,1] to [0,1]
    // from 0 to 1
    let t = 0.5 * (in.corner.x + 1.0);
    let ab = in.b - in.a;
    // safe normalize --- if length==0 fallback to (1,0)
    let ab_len = length(ab);
    var dir = vec2f(1.0, 0.0);
    if (ab_len > 0.0001) {
        dir = ab / ab_len;
    }
    let n = vec2<f32>(-dir.y, dir.x); // normal in pixel-space

    let added_radius = (length(ab) + in.radius)/length(ab);
    let rel_radius = in.radius/length(ab);

    let pos_px = in.a 
        - dir * in.radius 
        + ab * in.corner.x * added_radius *2.0
        - n * in.radius 
        + n * in.corner.y * in.radius * 2.0;

    out.frag_pos_px = pos_px;

    out.a_px = in.a;
    out.b_px = in.b;
    out.radius = in.radius;

    // convert pixel -> NDC (x: -1..1 left->right, y: -1..1 bottom->top)
    // pixel origin assumed top-left; map so (0,0) -> (-1, 1)
    let ndc_x = pos_px.x / u_screen.x * 2.0 - 1.0;
    let ndc_y = 1.0 - pos_px.y / u_screen.y * 2.0;
    out.clip = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);

    return out;
}

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

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let p = in.frag_pos_px;
    let d = ud_segment(p, in.a_px, in.b_px);

    // this should be replaced by radius later on I guess?
    let w = in.radius;
    // is the partial derivative of the current texel and the neighbouring ones in a 2x2 grid
    let aa = fwidth(d);

    var col = vec4(1.0) - smoothstep(w-aa, w, d);
    let fg = vec4(1.0);
    let bg = vec4(1.0, 1.0, 1.0, 0.0);

    col = mix(bg, fg, col); 
    return col;
}
