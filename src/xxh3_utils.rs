//!XXH3 related utilities
use crate::xxh3_common::*;

//Const version is only efficient when it is actually executed at compile time
#[inline(always)]
///Generates secret derived from provided seed and default secret.
///
///Efficient when executed at compile time as alternative to using version of algorithm with custom `seed`
pub const fn const_custom_default_secret(seed: u64) -> [u8; DEFAULT_SECRET_SIZE] {
    if seed == 0 {
        return DEFAULT_SECRET;
    }

    #[inline(always)]
    const fn read_u64(input: &[u8], cursor: usize) -> u64 {
        input[cursor] as u64
            | (input[cursor + 1] as u64) << 8
            | (input[cursor + 2] as u64) << 16
            | (input[cursor + 3] as u64) << 24
            | (input[cursor + 4] as u64) << 32
            | (input[cursor + 5] as u64) << 40
            | (input[cursor + 6] as u64) << 48
            | (input[cursor + 7] as u64) << 56
    }

    let mut idx = 0;
    let mut result = [0; DEFAULT_SECRET_SIZE];
    const NB_ROUNDS: usize = DEFAULT_SECRET_SIZE / 16;

    while idx < NB_ROUNDS {
        let lo = read_u64(&DEFAULT_SECRET, idx * 16).wrapping_add(seed).to_le_bytes();
        let hi = read_u64(&DEFAULT_SECRET, idx * 16 + 8).wrapping_sub(seed).to_le_bytes();

        result[idx * 16] = lo[0];
        result[idx * 16 + 1] = lo[1];
        result[idx * 16 + 2] = lo[2];
        result[idx * 16 + 3] = lo[3];
        result[idx * 16 + 4] = lo[4];
        result[idx * 16 + 5] = lo[5];
        result[idx * 16 + 6] = lo[6];
        result[idx * 16 + 7] = lo[7];

        result[idx * 16 + 8] = hi[0];
        result[idx * 16 + 8 + 1] = hi[1];
        result[idx * 16 + 8 + 2] = hi[2];
        result[idx * 16 + 8 + 3] = hi[3];
        result[idx * 16 + 8 + 4] = hi[4];
        result[idx * 16 + 8 + 5] = hi[5];
        result[idx * 16 + 8 + 6] = hi[6];
        result[idx * 16 + 8 + 7] = hi[7];

        idx += 1;
    }

    result
}
