import super::util;
import super::union_find;

struct Dimensions {
    columns: u32,
    rows: u32,
    _pad0: u32,
    _pad1: u32,
}

// group(0) binding(0) is in union-find
@group(0) @binding(1)
var<storage, read> infos: array<u32>;
@group(0) @binding(2)
var<uniform> dims : Dimensions;

@compute
@workgroup_size(8, 8, 1)
fn merge(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    // this needs to b *2 since it is supposed to be a 2x2 block
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    let labels_idx = row * dims.columns + col;

    if row < dims.rows && col < dims.columns {
        let info = infos[labels_idx];

        if util::HasBits(info, util::Q) {
            union_find::Union(labels_idx, labels_idx - (dims.columns * 2u));
        }

        if util::HasBits(info, util::R) {
            union_find::Union(labels_idx, labels_idx - (dims.columns * 2u) + 2u);
        }

        if util::HasBits(info, util::S) {
            union_find::Union(labels_idx, labels_idx - 2u);
        }
    }
}
