import super::util;

struct Dimensions {
    columns: u32,
    rows: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0)
var<storage, read_write> labels: array<u32>;
@group(0) @binding(1)
var<storage, read> infos: array<u32>;
@group(0) @binding(2)
var<uniform> dims : Dimensions;

@compute
@workgroup_size(8, 8, 1)
fn final_labeling(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    let img_row = dims.rows;
    let img_col = dims.columns;
    let labels_idx = row * img_col + col;
    let info = infos[labels_idx];

    if row < img_row && col < img_col {

        let label = labels[labels_idx] + 1u;
        if util::HasBits(info, util::A){
            labels[labels_idx] = label;
        } else {
            labels[labels_idx] = 0u;
        }
 
        if col + 1u < img_col {
            if util::HasBits(info, util::B){
                labels[labels_idx+1] = label;
            } else {
                labels[labels_idx+1] = 0u;
            }
        }

        if row + 1u < img_row {
            if util::HasBits(info, util::C){
                labels[labels_idx+img_col] = label;
            } else {
                labels[labels_idx+img_col] = 0u;
            }
        }

        if row +1u < img_row && col + 1u < img_col {
            if util::HasBits(info, util::D){
                labels[labels_idx+img_col+1u] = label;
            } else {
                labels[labels_idx+img_col+1u] = 0u;
            }
        }
    }
}
