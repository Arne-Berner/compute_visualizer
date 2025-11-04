import super::util;
// TODO labels could be 1/4 as small, because it labels for the whole 2x2 block
// what happens, if there is neither a,b,c or d?
// TODO Can I reuise in_image for labels?

const BIT0: u32 = 1u;
const BIT1: u32 = 1u << 1u;
const BIT2: u32 = 1u << 2u;
const BIT3: u32 = 1u << 3u;
const BIT4: u32 = 1u << 4u;
const BIT8: u32 = 1u << 8u;

@group(0) @binding(0)
var in_image: texture_storage_2d<rgba8uint, read>;
// only r32uint, r32float and r32sint can be read_write natively
@group(0) @binding(1)
var<storage, read_write> labels: array<u32>;
@group(0) @binding(2)
var<storage, read_write> infos: array<u32>;

fn texel_zero(col: u32, row: u32) -> bool {
    // textureLoad on storage texture requires in-bounds integer coords
    let v = textureLoad(in_image, vec2u(col,row));
    return v.r == 0u;
}
fn texel_nonzero(col: u32, row: u32, pos: vec2<i32>) -> bool {
    let load_pos = vec2i(i32(col), i32(row)) + pos;
    let v = textureLoad(in_image, load_pos);
    return v.r != 0u;
}

@compute
@workgroup_size(8, 8, 1)
fn init_labeling(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    let img_size = textureDimensions(in_image);
    let img_row = img_size.y;
    let img_col = img_size.x;
    let labels_idx = row * img_col + col;

    if row < img_row && col < img_col {

        //      | COL1  | COL2  | COL3  | COL4
        // ROW1 | P     | Q_1   | Q_2   | R
        // ROW2 | S_1   | a     | b     | 0
        // ROW3 | S_2   | c     | d     | 0
        //
        // Row 1: bits 0,1,2,3 correspond to col 1,2,3 and 4
        // Row 2: bits 4,5,6,7 correspond to col 1,2,3 and 4
        // Row 3: bits 8,9,10,11 correspond to col 1,2,3 and 4
        // only 0, 1, 2, 3, 4 and 8 are relevant and therefore set
        var P = 0x011Fu;

        // Bitmask representing two kinds of information
        // Bits 0, 1, 2, 3 are set if pixel a, b, c, d are foreground, respectively
        // Bits 4, 5, 6, 7 are set if block P, Q, R, S need to be merged to X in Merge phase
        // bits 0, 1, 2, 3 are set at the beginning and removed if necessary
        var info = 0x000Fu;

        // first row, so no row above it
        if row == 0u {
            P = util::RemoveBits(P, util::ROW_1);
        }

        // first column in a row, so no row to the left
        if col == 0u {
            P = util::RemoveBits(P, util::COL_1);
        }

        // labels/image rows start at 1, this makes it easier to reason about
        let row_num = row+1u;
        let col_num = col+1u;

        // There is no third row 
        if row_num + 1u > img_row {
            P = util::RemoveBits(P, util::ROW_3);
            info = util::RemoveBits(info, (util::C | util::D));
        }
        // there is no third and fourth col
        if col_num + 1u > img_col {
            P = util::RemoveBits(P, (util::COL_3 | util::COL_4));
            info = util::RemoveBits(info, (util::B | util::D));
        }

        if util::HasBits(info, util::A) {
            if texel_zero(col, row) {
                info = util::RemoveBits(info, (util::A));
            }
        }
        if util::HasBits(info, util::B) {
            if texel_zero(col+1, row) {
                info = util::RemoveBits(info, (util::B));
            }
        }
        if util::HasBits(info, util::C) {
            if texel_zero(col, row+1) {
                info = util::RemoveBits(info, (util::C));
            }
        }
        if util::HasBits(info, util::D) {
            if texel_zero(col+1, row+1) {
                info = util::RemoveBits(info, (util::D));
            }
        }

        if col_num + 2u > img_col {
            P = util::RemoveBits(P, util::COL_4);
        }

        // the first two are the x and y coordinates of the parent
        // the last one is the info
        var parent_idx = labels_idx;
        var parent_flag = false;

        // the 2x2 block "P"
        let p_pos = vec2i(-1,-1);
        if util::HasBits(P, BIT0) && texel_nonzero(col, row, p_pos) {
            parent_idx = labels_idx - (2u * img_col + 2u);
            parent_flag = true;
        }
        
        // the 2x2 block "Q"
        let q_pos_left = vec2i(0,-1);
        let q_pos_right = vec2i(1,-1);
        if (util::HasBits(P, BIT1) && texel_nonzero(col, row, q_pos_left)) || (util::HasBits(P, BIT2) && texel_nonzero(col, row, q_pos_right)) {
            if parent_flag {
                info = util::SetBits(info, util::Q);
            } else {
                // 2x2 to the top
                parent_idx = labels_idx - (2u * img_col);
                parent_flag = true;
            }
        }

        // the 2x2 block "R"
        let r_pos = vec2i(2, -1);
        if util::HasBits(P, BIT3) && texel_nonzero(col, row, r_pos) {
            if parent_flag {
                info = util::SetBits(info, util::R);
            } else {
                // 2x2 to the top-right
                parent_idx = labels_idx - (2u * img_col) + 2u;
                parent_flag = true;
            }
        }

        // the 2x2 block "S"
        let s_pos_top = vec2i(-1,0);
        let s_pos_bot = vec2i(-1,1);
        if (util::HasBits(P, BIT4) && texel_nonzero(col, row, s_pos_top)) || (util::HasBits(P, BIT8) && texel_nonzero(col, row, s_pos_bot)) {
            if parent_flag {
                info = util::SetBits(info, util::S);
            } else {
                parent_idx = labels_idx - 2u;
                parent_flag = true;
            }
        }
        
        // store parent position in current texel
        labels[labels_idx] = parent_idx;
        infos[labels_idx] = info;
    }
}
