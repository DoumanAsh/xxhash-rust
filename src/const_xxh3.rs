//!Xxh3 `const fn` implementation
//!
//!This module is efficient only when hashes are guaranteed to be executed at compile time.
//!At runtime algorithm is written in fairly inefficient code, although still fast enough.

use crate::xxh32_common as xxh32;
use crate::xxh64_common as xxh64;
use crate::xxh3_common::*;

const INITIAL_ACC: [u64; ACC_NB] = [
    xxh32::PRIME_3 as u64, xxh64::PRIME_1, xxh64::PRIME_2, xxh64::PRIME_3,
    xxh64::PRIME_4, xxh32::PRIME_2 as u64, xxh64::PRIME_5, xxh32::PRIME_1 as u64
];

#[inline(always)]
const fn read_u32(input: &[u8], cursor: usize) -> u32 {
    input[cursor] as u32 | (input[cursor + 1] as u32) << 8 | (input[cursor + 2] as u32) << 16 | (input[cursor + 3] as u32) << 24
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

#[inline]
const fn mix16_b(input: &[u8], input_offset: usize, secret: &[u8], secret_offset: usize, seed: u64) -> u64 {
    let mut input_lo = read_u64(input, input_offset);
    let mut input_hi = read_u64(input, input_offset + 8);

    input_lo ^= read_u64(secret, secret_offset).wrapping_add(seed);
    input_hi ^= read_u64(secret, secret_offset + 8).wrapping_sub(seed);

    mul128_fold64(input_lo, input_hi)
}

#[inline(always)]
const fn xxh3_64_9to16(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    let flip1 = (read_u64(secret, 24) ^ read_u64(secret, 32)).wrapping_add(seed);
    let flip2 = (read_u64(secret, 40) ^ read_u64(secret, 48)).wrapping_sub(seed);

    let input_lo = read_u64(input, 0) ^ flip1;
    let input_hi = read_u64(input, input.len() - 8) ^ flip2;

    let acc = (input.len() as u64).wrapping_add(input_lo.swap_bytes())
                                  .wrapping_add(input_hi)
                                  .wrapping_add(mul128_fold64(input_lo, input_hi));

    avalanche(acc)
}

#[inline(always)]
const fn xxh3_64_4to8(input: &[u8], mut seed: u64, secret: &[u8]) -> u64 {
    seed ^= ((seed as u32).swap_bytes() as u64) << 32;

    let input1 = read_u32(input, 0);
    let input2 = read_u32(input, input.len() - 4);

    let flip = (read_u64(secret, 8) ^ read_u64(secret, 16)).wrapping_sub(seed);
    let input64 = (input2 as u64).wrapping_add((input1 as u64) << 32);
    let keyed = input64 ^ flip;

    strong_avalanche(keyed, input.len() as u64)
}

#[inline(always)]
const fn xxh3_64_1to3(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    let combo = ((input[0] as u32) << 16)
                | ((input[input.len() >> 1] as u32) << 24)
                | (input[input.len() - 1] as u32)
                | ((input.len() as u32) << 8);


    let flip = ((read_u32(secret, 0) ^ read_u32(secret, 4)) as u64).wrapping_add(seed);
    xxh64::avalanche((combo as u64) ^ flip)
}

#[inline(always)]
const fn xxh3_64_0to16(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    if input.len() > 8 {
        xxh3_64_9to16(input, seed, secret)
    } else if input.len() >= 4 {
        xxh3_64_4to8(input, seed, secret)
    } else if input.len() > 0 {
        xxh3_64_1to3(input, seed, secret)
    } else {
        xxh64::avalanche(seed ^ read_u64(secret, 56) ^ read_u64(secret, 64))
    }
}

#[inline(always)]
const fn xxh3_64_7to128(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    let mut acc = (input.len() as u64).wrapping_mul(xxh64::PRIME_1);

    if input.len() > 32 {
        if input.len() > 64 {
            if input.len() > 96 {
                acc = acc.wrapping_add(mix16_b(input, 48, secret, 96, seed));
                acc = acc.wrapping_add(mix16_b(input, input.len()-64, secret, 112, seed));
            }

            acc = acc.wrapping_add(mix16_b(input, 32, secret, 64, seed));
            acc = acc.wrapping_add(mix16_b(input, input.len()-48, secret, 80, seed));
        }

        acc = acc.wrapping_add(mix16_b(input, 16, secret, 32, seed));
        acc = acc.wrapping_add(mix16_b(input, input.len()-32, secret, 48, seed));
    }

    acc = acc.wrapping_add(mix16_b(input, 0, secret, 0, seed));
    acc = acc.wrapping_add(mix16_b(input, input.len()-16, secret, 16, seed));

    avalanche(acc)
}

const fn xxh3_64_129to240(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    const START_OFFSET: usize = 3;
    const LAST_OFFSET: usize = 17;

    let mut acc = (input.len() as u64).wrapping_mul(xxh64::PRIME_1);
    let nb_rounds = input.len() / 16;

    let mut idx = 0;
    while idx < 8 {
        acc = acc.wrapping_add(mix16_b(input, 16*idx, secret, 16*idx, seed));
        idx += 1;
    }
    acc = avalanche(acc);

    while idx < nb_rounds {
        acc = acc.wrapping_add(mix16_b(input, 16*idx, secret, 16*(idx-8)+START_OFFSET, seed));
        idx += 1;
    }

    acc = acc.wrapping_add(mix16_b(input, input.len()-16, secret, SECRET_SIZE_MIN-LAST_OFFSET, seed));

    avalanche(acc)
}

#[inline(always)]
const fn mix_two_accs(acc: &[u64], acc_offset: usize, secret: &[u8], secret_offset: usize) -> u64 {
    mul128_fold64(acc[acc_offset] ^ read_u64(secret, secret_offset),
                  acc[acc_offset + 1] ^ read_u64(secret, secret_offset + 8))
}

#[inline]
const fn merge_accs(acc: &[u64], secret: &[u8], secret_offset: usize, mut result: u64) -> u64 {
    let mut idx = 0;
    while idx < 4 {
        result = result.wrapping_add(mix_two_accs(acc, idx * 2, secret, secret_offset + idx * 16));
        idx += 1;
    }

    avalanche(result)
}

const fn scramble_acc(mut acc: [u64; ACC_NB], secret: &[u8], secret_offset: usize) -> [u64; ACC_NB] {
    let mut idx = 0;

    while idx < ACC_NB {
        let key = read_u64(secret, secret_offset + 8 * idx);
        let mut acc_val = xorshift64(acc[idx], 47);
        acc_val ^= key;
        acc[idx] = acc_val.wrapping_mul(xxh32::PRIME_1 as u64);

        idx += 1;
    }

    acc
}

const fn accumulate_512(mut acc: [u64; ACC_NB], input: &[u8], input_offset: usize, secret: &[u8], secret_offset: usize) -> [u64; ACC_NB] {
    #[inline(always)]
    const fn mult32_to64(left: u32, right: u32) -> u64 {
        (left as u64).wrapping_mul(right as u64)
    }

    let mut idx = 0;
    while idx < ACC_NB {
        let data_val = read_u64(input, input_offset + 8 * idx);
        let data_key = data_val ^ read_u64(secret, secret_offset + 8 * idx);

        acc[idx ^ 1] = acc[idx ^ 1].wrapping_add(data_val);
        acc[idx] = acc[idx].wrapping_add(mult32_to64((data_key & 0xFFFFFFFF) as u32, (data_key >> 32) as u32));

        idx += 1;
    }

    acc
}

#[inline(always)]
const fn accumulate_loop(mut acc: [u64; ACC_NB], input: &[u8], input_offset: usize, secret: &[u8], secret_offset: usize, nb_stripes: usize) -> [u64; ACC_NB] {
    let mut idx = 0;
    while idx < nb_stripes {
        acc = accumulate_512(acc, input, input_offset + idx * STRIPE_LEN, secret, secret_offset + idx * SECRET_CONSUME_RATE);

        idx += 1;
    }

    acc
}

#[inline]
const fn hash_long_internal_loop(input: &[u8], secret: &[u8]) -> [u64; ACC_NB] {
    let mut acc = INITIAL_ACC;

    let nb_stripes = (secret.len() - STRIPE_LEN) / SECRET_CONSUME_RATE;
    let block_len = STRIPE_LEN * nb_stripes;
    let nb_blocks = (input.len() - 1) / block_len;

    let mut idx = 0;
    while idx < nb_blocks {
        acc = accumulate_loop(acc, input, idx * block_len, secret, 0, nb_stripes);
        acc = scramble_acc(acc, secret, secret.len() - STRIPE_LEN);

        idx += 1;
    }

    let nb_stripes = ((input.len() - 1) - (block_len * nb_blocks)) / STRIPE_LEN;

    acc = accumulate_loop(acc, input, nb_blocks * block_len, secret, 0, nb_stripes);
    accumulate_512(acc, input, input.len() - STRIPE_LEN, secret, secret.len() - STRIPE_LEN - SECRET_LASTACC_START)
}

const fn xxh3_64_long_impl(input: &[u8], secret: &[u8]) -> u64 {
    let acc = hash_long_internal_loop(input, secret);

    merge_accs(&acc, secret, SECRET_MERGEACCS_START, (input.len() as u64).wrapping_mul(xxh64::PRIME_1))
}

#[inline(always)]
///Returns 64bit hash for provided input.
pub const fn xxh3_64(input: &[u8]) -> u64 {
    xxh3_64_with_seed(input, 0)
}

///Returns 64bit hash for provided input using seed.
pub const fn xxh3_64_with_seed(input: &[u8], seed: u64) -> u64 {
    if input.len() <= 16 {
        xxh3_64_0to16(input, seed, &DEFAULT_SECRET)
    } else if input.len() <= 128 {
        xxh3_64_7to128(input, seed, &DEFAULT_SECRET)
    } else if input.len() <= MID_SIZE_MAX {
        xxh3_64_129to240(input, seed, &DEFAULT_SECRET)
    } else {
        xxh3_64_long_impl(input, &const_custom_default_secret(seed))
    }
}

///Returns 64bit hash for provided input using custom secret.
pub const fn xxh3_64_with_secret(input: &[u8], secret: &[u8; DEFAULT_SECRET_SIZE]) -> u64 {
    if input.len() <= 16 {
        xxh3_64_0to16(input, 0, secret)
    } else if input.len() <= 128 {
        xxh3_64_7to128(input, 0, secret)
    } else if input.len() <= MID_SIZE_MAX {
        xxh3_64_129to240(input, 0, secret)
    } else {
        xxh3_64_long_impl(input, secret)
    }
}

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
