alias Info = u32;


const COL_1:u32 = 0x1111u;
const COL_2:u32 = 0x2222u;
const COL_3:u32 = 0x4444u;
const COL_4:u32 = 0x8888u;
const ROW_1:u32 = 0x000Fu;
const ROW_2:u32 = 0x00F0u;
const ROW_3:u32 = 0x0F00u;
const A: Info = 1u;
const B: Info = 1u << 1u;
const C: Info = 1u << 2u;
const D: Info = 1u << 3u;
const P: Info = 1u << 4u;
const Q: Info = 1u << 5u;
const R: Info = 1u << 6u;
const S: Info = 1u << 7u;

fn HasBits(mask: u32, bits: u32) -> bool {
  return (mask & bits) != 0u;
}

fn SetBits(mask: u32, bits: u32) -> u32 {
  return mask | bits;
}

fn RemoveBits(mask: u32, bits: u32) -> u32 {
  return mask & (~bits);
}
