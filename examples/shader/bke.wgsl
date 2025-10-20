const BLOCK_ROWS = 16;
const BLOCK_COLS = 16;


// this should have the foreground pixel info
// TODO make this rgba8uint?
@group(0) @binding(0)
var in_image: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1)
var out_image: texture_storage_2d<rgba8unorm, write>;
// this should be empty at the beginning
/*
@group(0) @binding(2)
var labels: texture_storage_2d<rg32uint, write>; // using slightly bigger adress spaces than necessary
*/

@group(1) @binding(0)
var<storage, read> a_s_buf: array<atomic<u32>>;
// TODO this can't be run on webgpu and needs a flag
@group(1) @binding(1)
var<storage, read_write> s_buf: array<u32>;

// Optional alias for readability
alias Info = u32;

// Bit positions
const Info_a: Info = 0u;
const Info_b: Info = 1u;
const Info_c: Info = 2u;
const Info_d: Info = 3u;
const Info_P: Info = 4u;
const Info_Q: Info = 5u;
const Info_R: Info = 6u;
const Info_S: Info = 7u;

fn HasBit(bitmap: u32, pos: Info) -> u32 {
  return (bitmap >> pos) & 1u;
} 

fn SetBit(bitmap: u32, pos: Info) -> u32 {
  return bitmap | (1u << pos);
}

/// Find goes through the parent nodes recursively, until it finds the root node.
/// This works, because root points to itself. Usually this would be done in place,
/// but wgsl does not allow mutable function parameters.
fn Find(n: u32) -> u32 {
    var idx = n;

    loop {
        let parent = atomicLoad(&a_s_buf[idx]);
        if (parent == idx) {
            break;
        }
        idx = parent;

    }
    return idx;
}

/// This is not using atomics, so that it can run really fast.
fn FindAndCompress(n: u32) -> u32 {
    var idx = n;
    while (s_buf[idx] != idx) {
        idx = s_buf[idx];
        s_buf[n] = idx;
    }
    return idx;
}

fn Union(a_in: u32, b_in: u32) {
    var a: u32 = a_in;
    var b: u32 = b_in;
    var done: bool = false;

    loop {
        // Find current roots
        a = Find(a);
        b = Find(b);

        if (a < b) {
            // atomicMin takes i32 and returns the previous i32 value
            let old = atomicMin(&a_s_buf[b], a);
            if old == b {
                break;
            }
            b = old;
        } else if (b < a) {
            let old = atomicMin(&a_s_buf[a], b);
            if old == a {
                break;
            }
            a = old;
        } else {
            break;
        }
    }
}


@compute
@workgroup_size(6, 6, 1)
fn init_labeling(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    // this needs to b *2 since it is supposed to be a 2x2 block
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    // let img_idx = row * textureDimensions(image).x + col;
    // let labels_idx = row * textureDimensions(labels).x + col;
    let img_idx = row * textureDimensions(in_image) + col; // global_invocation_index
    let labels_idx = row * (labelInfo.stride_bytes/ labelInfo.elem_size)  + col; // global_invocation_index

    if row < labelInfo.row && col < labelInfo.col {
        // TODO since I _have_ to use u32, I can probably put the info about the pixel into the labelinfo
        // TODO I might also be able to put the img and label info into one struct?
        // It does not make sense, to use one pixel for multiple threads, since atomics would make it slow
        var P = 0u;
        // Bitmask representing two kinds of information
        // Bits 0, 1, 2, 3 are set if pixel a, b, c, d are foreground, respectively
        // Bits 4, 5, 6, 7 are set if block P, Q, R, S need to be merged to X in Merge phase
        var info = 0u;
        

    }
}


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
    let color: vec4<f32> = tex;

    // maybe get the image width via a uniform buffer
    textureStore(out_tex, vec2<i32>(gid.xy), color);
}


// minimum uniform struct alignment=16
// https://gpuweb.github.io/gpuweb/wgsl/#address-space-layout-constraints
/*
struct Image2DInfo {
  column: u32,        
  row: u32,       
  stride_bytes: u32,
  elem_size: u32, // 1 byte per pixel and 4 bytes per label    
};
*/
