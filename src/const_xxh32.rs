//!Const eval friendly xxh32 implementation.

use crate::xxh32_common::*;
use crate::utils::slice_chunks;

const fn finalize(mut input: u32, data: &[u8]) -> u32 {
    let (chunks, remainder) = slice_chunks::<4>(data);

    let mut idx = 0;
    while idx < chunks.len() {
        let chunk = &chunks[idx];
        input = input.wrapping_add(
            u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_le().wrapping_mul(PRIME_3)
        );
        input = input.rotate_left(17).wrapping_mul(PRIME_4);
        idx += 1;
    }

    idx = 0;
    while idx < remainder.len() {
        input = input.wrapping_add((remainder[idx] as u32).wrapping_mul(PRIME_5));
        input = input.rotate_left(11).wrapping_mul(PRIME_1);
        idx += 1;
    }

    avalanche(input)
}

///Const variant of xxh32 hashing
pub const fn xxh32(input: &[u8], seed: u32) -> u32 {
    let mut result = input.len() as u32;

    if input.len() >= CHUNK_SIZE {
        let mut v1 = seed.wrapping_add(PRIME_1).wrapping_add(PRIME_2);
        let mut v2 = seed.wrapping_add(PRIME_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(PRIME_1);

        let (chunks, remainder) = slice_chunks::<CHUNK_SIZE>(input);

        let mut idx = 0;
        while idx < chunks.len() {
            let chunk = &chunks[idx];
            v1 = round(v1, u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_le());
            v2 = round(v2, u32::from_ne_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]).to_le());
            v3 = round(v3, u32::from_ne_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]).to_le());
            v4 = round(v4, u32::from_ne_bytes([chunk[12], chunk[13], chunk[14], chunk[15]]).to_le());

            idx += 1;
        }

        result = result.wrapping_add(
            v1.rotate_left(1).wrapping_add(
                v2.rotate_left(7).wrapping_add(
                    v3.rotate_left(12).wrapping_add(
                        v4.rotate_left(18)
                    )
                )
            )
        );
        finalize(result, remainder)
    } else {
        result = result.wrapping_add(seed.wrapping_add(PRIME_5));
        finalize(result, input)
    }
}
