//!Const 64 bit version of xxhash algorithm

use crate::xxh64_common::*;
use crate::utils::slice_chunks;

const fn finalize(mut input: u64, data: &[u8]) -> u64 {
    let (chunks, remainder) = slice_chunks::<8>(data);

    let mut idx = 0;
    while idx < chunks.len() {
        let chunk = &chunks[idx];
        input ^= round(0, u64::from_ne_bytes(*chunk).to_le());
        input = input.rotate_left(27).wrapping_mul(PRIME_1).wrapping_add(PRIME_4);
        idx += 1;
    }

    idx = 0;
    let (chunks, remainder) = slice_chunks::<4>(remainder);
    while idx < chunks.len() {
        let chunk = &chunks[idx];
        input ^= (u32::from_ne_bytes(*chunk).to_le() as u64).wrapping_mul(PRIME_1);
        input = input.rotate_left(23).wrapping_mul(PRIME_2).wrapping_add(PRIME_3);
        idx += 1;
    }

    idx = 0;
    while idx < remainder.len() {
        input ^= (remainder[idx] as u64).wrapping_mul(PRIME_5);
        input = input.rotate_left(11).wrapping_mul(PRIME_1);
        idx += 1;
    }

    avalanche(input)
}

///Returns hash for the provided input.
pub const fn xxh64(input: &[u8], seed: u64) -> u64 {
    let input_len = input.len() as u64;
    let mut result;

    if input.len() >= CHUNK_SIZE {
        let mut v1 = seed.wrapping_add(PRIME_1).wrapping_add(PRIME_2);
        let mut v2 = seed.wrapping_add(PRIME_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(PRIME_1);

        let mut idx = 0;
        let (chunks, remainder) = slice_chunks::<CHUNK_SIZE>(input);
        while idx < chunks.len() {
            let chunk = &chunks[idx];
            v1 = round(v1, u64::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]]).to_le());
            v2 = round(v2, u64::from_ne_bytes([chunk[8], chunk[9], chunk[10], chunk[11], chunk[12], chunk[13], chunk[14], chunk[15]]).to_le());
            v3 = round(v3, u64::from_ne_bytes([chunk[16], chunk[17], chunk[18], chunk[19], chunk[20], chunk[21], chunk[22], chunk[23]]).to_le());
            v4 = round(v4, u64::from_ne_bytes([chunk[24], chunk[25], chunk[26], chunk[27], chunk[28], chunk[29], chunk[30], chunk[31]]).to_le());

            idx += 1;
        }

        result = v1.rotate_left(1).wrapping_add(v2.rotate_left(7))
                                  .wrapping_add(v3.rotate_left(12))
                                  .wrapping_add(v4.rotate_left(18));

        result = merge_round(result, v1);
        result = merge_round(result, v2);
        result = merge_round(result, v3);
        result = merge_round(result, v4);
        finalize(result.wrapping_add(input_len), remainder)
    } else {
        finalize(seed.wrapping_add(PRIME_5).wrapping_add(input_len), input)
    }
}
