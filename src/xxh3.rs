//!XXH3 implementation
//!
//!Provides only 64bit variant as most usable one.

use core::{ptr, mem};

use crate::xxh32_common as xxh32;
use crate::xxh64_common as xxh64;

// Code is as close to original C implementation as possible
// It does make it look ugly, but it is fast and easy to update once xxhash gets new version.

const STRIPE_LEN: usize = 64;
const SECRET_CONSUME_RATE: usize = 8;
const ACC_NB: usize = STRIPE_LEN / mem::size_of::<u64>();

const SECRET_MERGEACCS_START: usize = 11;
const SECRET_LASTACC_START: usize = 7;  //not aligned on 8, last secret is different from acc & scrambler

const MID_SIZE_MAX: usize = 240;
const SECRET_SIZE_MIN: usize = 136;
const DEFAULT_SECRET_SIZE: usize = 192;
const DEFAULT_SECRET_LIMIT: usize = DEFAULT_SECRET_SIZE / STRIPE_LEN;
const DEFAULT_SECRET: [u8; DEFAULT_SECRET_SIZE] = [
    0xb8, 0xfe, 0x6c, 0x39, 0x23, 0xa4, 0x4b, 0xbe, 0x7c, 0x01, 0x81, 0x2c, 0xf7, 0x21, 0xad, 0x1c,
    0xde, 0xd4, 0x6d, 0xe9, 0x83, 0x90, 0x97, 0xdb, 0x72, 0x40, 0xa4, 0xa4, 0xb7, 0xb3, 0x67, 0x1f,
    0xcb, 0x79, 0xe6, 0x4e, 0xcc, 0xc0, 0xe5, 0x78, 0x82, 0x5a, 0xd0, 0x7d, 0xcc, 0xff, 0x72, 0x21,
    0xb8, 0x08, 0x46, 0x74, 0xf7, 0x43, 0x24, 0x8e, 0xe0, 0x35, 0x90, 0xe6, 0x81, 0x3a, 0x26, 0x4c,
    0x3c, 0x28, 0x52, 0xbb, 0x91, 0xc3, 0x00, 0xcb, 0x88, 0xd0, 0x65, 0x8b, 0x1b, 0x53, 0x2e, 0xa3,
    0x71, 0x64, 0x48, 0x97, 0xa2, 0x0d, 0xf9, 0x4e, 0x38, 0x19, 0xef, 0x46, 0xa9, 0xde, 0xac, 0xd8,
    0xa8, 0xfa, 0x76, 0x3f, 0xe3, 0x9c, 0x34, 0x3f, 0xf9, 0xdc, 0xbb, 0xc7, 0xc7, 0x0b, 0x4f, 0x1d,
    0x8a, 0x51, 0xe0, 0x4b, 0xcd, 0xb4, 0x59, 0x31, 0xc8, 0x9f, 0x7e, 0xc9, 0xd9, 0x78, 0x73, 0x64,
    0xea, 0xc5, 0xac, 0x83, 0x34, 0xd3, 0xeb, 0xc3, 0xc5, 0x81, 0xa0, 0xff, 0xfa, 0x13, 0x63, 0xeb,
    0x17, 0x0d, 0xdd, 0x51, 0xb7, 0xf0, 0xda, 0x49, 0xd3, 0x16, 0x55, 0x26, 0x29, 0xd4, 0x68, 0x9e,
    0x2b, 0x16, 0xbe, 0x58, 0x7d, 0x47, 0xa1, 0xfc, 0x8f, 0xf8, 0xb8, 0xd1, 0x7a, 0xd0, 0x31, 0xce,
    0x45, 0xcb, 0x3a, 0x8f, 0x95, 0x16, 0x04, 0x28, 0xaf, 0xd7, 0xfb, 0xca, 0xbb, 0x4b, 0x40, 0x7e,
];

#[cfg(target_feature = "sse2")]
#[repr(align(16))]
#[derive(Clone)]
struct Acc([u64; ACC_NB]);
#[cfg(not(all(target_feature = "sse2")))]
#[repr(align(8))]
#[derive(Clone)]
struct Acc([u64; ACC_NB]);

const INITIAL_ACC: Acc = Acc([
    xxh32::PRIME_3 as u64, xxh64::PRIME_1, xxh64::PRIME_2, xxh64::PRIME_3,
    xxh64::PRIME_4, xxh32::PRIME_2 as u64, xxh64::PRIME_5, xxh32::PRIME_1 as u64
]);

type LongHashFn = fn(&[u8], u64, &[u8]) -> u64;

#[cfg(target_feature = "sse2")]
#[inline]
const fn _mm_shuffle(z: u32, y: u32, x: u32, w: u32) -> i32 {
    ((z << 6) | (y << 4) | (x << 2) | w) as i32
}

#[inline(always)]
fn _mm_prefetch(ptr: *const i8, offset: isize) {
    #[cfg(target_arch = "x86")]
    unsafe {
        core::arch::x86::_mm_prefetch(ptr.offset(offset), core::arch::x86::_MM_HINT_T0);
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_mm_prefetch(ptr.offset(offset), core::arch::x86_64::_MM_HINT_T0);
    }
}

#[inline(always)]
///It is faster to use unsafe offset than wasting time to slice
fn slice_offset_ptr(slice: &[u8], offset: usize) -> *const u8 {
    debug_assert!(slice.len() >= offset);

    unsafe {
        slice.as_ptr().add(offset)
    }
}

#[inline(always)]
fn read_32le_unaligned(data: *const u8) -> u32 {
    let mut result = mem::MaybeUninit::<u32>::uninit();
    unsafe {
        ptr::copy_nonoverlapping(data, result.as_mut_ptr() as _, mem::size_of::<u32>());
        result.assume_init().to_le()
    }
}

#[inline(always)]
fn read_64le_unaligned(data: *const u8) -> u64 {
    let mut result = mem::MaybeUninit::<u64>::uninit();
    unsafe {
        ptr::copy_nonoverlapping(data, result.as_mut_ptr() as _, mem::size_of::<u64>());
        result.assume_init().to_le()
    }
}

#[inline]
const fn xorshift64(value: u64, shift: u64) -> u64 {
    value ^ (value >> shift)
}

#[inline]
const fn avalanche(mut value: u64) -> u64 {
    value = xorshift64(value, 37);
    value = value.wrapping_mul(0x165667919E3779F9);
    xorshift64(value, 32)
}

#[inline]
const fn strong_avalanche(mut value: u64, len: u64) -> u64 {
    value ^= value.rotate_left(49) ^ value.rotate_left(24);
    value = value.wrapping_mul(0x9FB21C651E98DF25);
    value ^= (value >> 35).wrapping_add(len);
    value = value.wrapping_mul(0x9FB21C651E98DF25);
    xorshift64(value, 28)
}

#[inline]
const fn mul64_to128(left: u64, right: u64) -> (u64, u64) {
    let product = left as u128 * right as u128;
    (product as u64, (product >> 64) as u64)
}

#[inline]
const fn mul128_fold64(left: u64, right: u64) -> u64 {
    let (low, high) = mul64_to128(left, right);
    low ^ high
}

#[inline(always)]
fn mix_two_accs(acc: &mut [u64], secret: *const u8) -> u64 {
    mul128_fold64(acc[0] ^ read_64le_unaligned(secret),
                  acc[1] ^ read_64le_unaligned(unsafe { secret.offset(8) }))
}

#[inline]
fn merge_accs(acc: &mut Acc, secret: *const u8, mut result: u64) -> u64 {
    for idx in 0..4 {
        result = result.wrapping_add(mix_two_accs(&mut acc.0[idx * 2..], unsafe { secret.add(idx * 16) }));
    }

    avalanche(result)
}

#[inline]
fn mix16_b(input: *const u8, secret: *const u8, seed: u64) -> u64 {
    let mut input_lo = read_64le_unaligned(input);
    let mut input_hi = read_64le_unaligned(unsafe { input.offset(8) });

    input_lo ^= read_64le_unaligned(secret).wrapping_add(seed);
    input_hi ^= read_64le_unaligned(unsafe { secret.offset(8) }).wrapping_sub(seed);

    mul128_fold64(input_lo, input_hi)
}

#[inline(always)]
fn custom_default_secret(seed: u64) -> [u8; DEFAULT_SECRET_SIZE] {
    let mut result = mem::MaybeUninit::<[u8; DEFAULT_SECRET_SIZE]>::uninit();

    let nb_rounds = DEFAULT_SECRET_SIZE / 16;

    for idx in 0..nb_rounds {
        let low = read_64le_unaligned(slice_offset_ptr(&DEFAULT_SECRET, idx * 16)).wrapping_add(seed);
        let hi = read_64le_unaligned(slice_offset_ptr(&DEFAULT_SECRET, idx * 16 + 8)).wrapping_sub(seed);

        unsafe {
            ptr::copy_nonoverlapping(low.to_le_bytes().as_ptr(), (result.as_mut_ptr() as *mut u8).add(idx * 16), mem::size_of::<u64>());
            ptr::copy_nonoverlapping(hi.to_le_bytes().as_ptr(), (result.as_mut_ptr() as *mut u8).add(idx * 16 + 8), mem::size_of::<u64>());
        }
    }

    unsafe {
        result.assume_init()
    }
}

//Const version is only efficient when it is actually executed at runtime
#[inline(always)]
const fn const_custom_default_secret(seed: u64) -> [u8; DEFAULT_SECRET_SIZE] {
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

//TODO: Should we add AVX?
//      SSE is safe cuz it is available everywhere, but avx should probably be optional
fn accumulate_512(acc: &mut Acc, input: *const u8, secret: *const u8) {
    #[cfg(target_feature = "sse2")]
    unsafe {
        #[cfg(target_arch = "x86")]
        use core::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::*;

        let xacc = acc.0.as_mut_ptr() as *mut __m128i;
        let xinput = input as *const __m128i;
        let xsecret = secret as *const __m128i;

        for idx in 0..STRIPE_LEN / mem::size_of::<__m128i>() {
            let data_vec = _mm_loadu_si128(xinput.add(idx));
            let key_vec = _mm_loadu_si128(xsecret.add(idx));
            let data_key = _mm_xor_si128(data_vec, key_vec);

            let data_key_lo = _mm_shuffle_epi32(data_key, _mm_shuffle(0, 3, 0, 1));
            let product = _mm_mul_epu32(data_key, data_key_lo);

            let data_swap = _mm_shuffle_epi32(data_vec, _mm_shuffle(1,0,3,2));
            let sum = _mm_add_epi64(*xacc.add(idx), data_swap);
            xacc.add(idx).write(_mm_add_epi64(product, sum));
        }
    }

    #[cfg(not(all(target_feature = "sse2")))]
    {
        #[inline(always)]
        const fn mult32_to64(left: u32, right: u32) -> u64 {
            (left as u64).wrapping_mul(right as u64)
        }

        for idx in 0..ACC_NB {
            let data_val = read_64le_unaligned(unsafe  { input.add(8 * idx) });
            let data_key = data_val ^ read_64le_unaligned(unsafe { secret.add(8 * idx) });

            acc.0[idx ^ 1] = acc.0[idx ^ 1].wrapping_add(data_val);
            acc.0[idx] = acc.0[idx].wrapping_add(mult32_to64((data_key & 0xFFFFFFFF) as u32, (data_key >> 32) as u32));
        }
    }
}

fn scramble_acc(acc: &mut Acc, secret: *const u8) {
    #[cfg(target_feature = "sse2")]
    unsafe {
        #[cfg(target_arch = "x86")]
        use core::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::*;

        let xacc = acc.0.as_mut_ptr() as *mut __m128i;
        let xsecret = secret as *const __m128i;
        let prime32 = _mm_set1_epi32(xxh32::PRIME_1 as i32);

        for idx in 0..STRIPE_LEN / mem::size_of::<__m128i>() {
            let acc_vec = *xacc.add(idx);
            let shifted = _mm_srli_epi64(acc_vec, 47);
            let data_vec = _mm_xor_si128(acc_vec, shifted);

            let key_vec = _mm_loadu_si128(xsecret.add(idx));
            let data_key = _mm_xor_si128(data_vec, key_vec);

            let data_key_hi = _mm_shuffle_epi32(data_key, _mm_shuffle(0, 3, 0, 1));
            let prod_lo = _mm_mul_epu32(data_key, prime32);
            let prod_hi = _mm_mul_epu32(data_key_hi, prime32);
            xacc.add(idx).write(_mm_add_epi64(prod_lo, _mm_slli_epi64(prod_hi, 32)));
        }
    }

    #[cfg(not(all(target_feature = "sse2")))]
    {
        for idx in 0..ACC_NB {
            let key = read_64le_unaligned(unsafe { secret.add(8 * idx) });
            let mut acc_val = xorshift64(acc.0[idx], 47);
            acc_val ^= key;
            acc.0[idx] = acc_val.wrapping_mul(xxh32::PRIME_1 as u64);
        }
    }
}

#[inline(always)]
fn accumulate_loop(acc: &mut Acc, input: *const u8, secret: *const u8, nb_stripes: usize) {
    for idx in 0..nb_stripes {
        _mm_prefetch(input as _, 320);
        accumulate_512(acc, unsafe { input.add(idx * STRIPE_LEN) }, unsafe { secret.add(idx * SECRET_CONSUME_RATE) });
    }
}

#[inline]
fn hash_long_internal_loop(acc: &mut Acc, input: &[u8], secret: &[u8]) {
    let nb_stripes = (secret.len() - STRIPE_LEN) / SECRET_CONSUME_RATE;
    let block_len = STRIPE_LEN * nb_stripes;
    let nb_blocks = (input.len() - 1) / block_len;

    for idx in 0..nb_blocks {
        accumulate_loop(acc, slice_offset_ptr(input, idx * block_len), secret.as_ptr(), nb_stripes);
        scramble_acc(acc, slice_offset_ptr(secret, secret.len() - STRIPE_LEN));
    }

    //last partial block
    debug_assert!(input.len() > STRIPE_LEN);

    let nb_stripes = ((input.len() - 1) - (block_len * nb_blocks)) / STRIPE_LEN;
    debug_assert!(nb_stripes <= (secret.len() / SECRET_CONSUME_RATE));
    accumulate_loop(acc, slice_offset_ptr(input, nb_blocks * block_len), secret.as_ptr(), nb_stripes);

    //last stripe
    accumulate_512(acc, slice_offset_ptr(input, input.len() - STRIPE_LEN), slice_offset_ptr(secret, secret.len() - STRIPE_LEN - SECRET_LASTACC_START));
}

#[inline(always)]
fn xxh3_64_1to3(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    debug_assert!(input.len() >= 1 && input.len() <= 3);
    let combo = ((input[0] as u32) << 16)
                | ((input[input.len() >> 1] as u32) << 24)
                | (input[input.len() - 1] as u32)
                | ((input.len() as u32) << 8);


    let flip = ((read_32le_unaligned(secret.as_ptr()) ^ read_32le_unaligned(slice_offset_ptr(secret, 4))) as u64).wrapping_add(seed);
    xxh64::avalanche((combo as u64) ^ flip)
}

#[inline(always)]
fn xxh3_64_4to8(input: &[u8], mut seed: u64, secret: &[u8]) -> u64 {
    debug_assert!(input.len() >= 4 && input.len() <= 8);

    seed ^= ((seed as u32).swap_bytes() as u64) << 32;

    let input1 = read_32le_unaligned(input.as_ptr());
    let input2 = read_32le_unaligned(slice_offset_ptr(input, input.len() - 4));

    let flip = (read_64le_unaligned(slice_offset_ptr(secret, 8)) ^ read_64le_unaligned(slice_offset_ptr(secret, 16))).wrapping_sub(seed);
    let input64 = (input2 as u64).wrapping_add((input1 as u64) << 32);
    let keyed = input64 ^ flip;

    strong_avalanche(keyed, input.len() as u64)
}

#[inline(always)]
fn xxh3_64_9to16(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    debug_assert!(input.len() >= 9 && input.len() <= 16);

    let flip1 = (read_64le_unaligned(slice_offset_ptr(secret, 24)) ^ read_64le_unaligned(slice_offset_ptr(secret, 32))).wrapping_add(seed);
    let flip2 = (read_64le_unaligned(slice_offset_ptr(secret, 40)) ^ read_64le_unaligned(slice_offset_ptr(secret, 48))).wrapping_sub(seed);

    let input_lo = read_64le_unaligned(input.as_ptr()) ^ flip1;
    let input_hi = read_64le_unaligned(slice_offset_ptr(input, input.len() - 8)) ^ flip2;

    let acc = (input.len() as u64).wrapping_add(input_lo.swap_bytes())
                                  .wrapping_add(input_hi)
                                  .wrapping_add(mul128_fold64(input_lo, input_hi));

    avalanche(acc)
}

#[inline(always)]
fn xxh3_64_0to16(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    if input.len() > 8 {
        xxh3_64_9to16(input, seed, secret)
    } else if input.len() >= 4 {
        xxh3_64_4to8(input, seed, secret)
    } else if input.len() > 0 {
        xxh3_64_1to3(input, seed, secret)
    } else {
        xxh64::avalanche(seed ^ (read_64le_unaligned(slice_offset_ptr(secret, 56)) ^ read_64le_unaligned(slice_offset_ptr(secret, 64))))
    }
}

#[inline(always)]
fn xxh3_64_7to128(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    let mut acc = (input.len() as u64).wrapping_mul(xxh64::PRIME_1);

    if input.len() > 32 {
        if input.len() > 64 {
            if input.len() > 96 {
                acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, 48), slice_offset_ptr(secret, 96), seed));
                acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, input.len()-64), slice_offset_ptr(secret, 112), seed));
            }

            acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, 32), slice_offset_ptr(secret, 64), seed));
            acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, input.len()-48), slice_offset_ptr(secret, 80), seed));
        }

        acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, 16), slice_offset_ptr(secret, 32), seed));
        acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, input.len()-32), slice_offset_ptr(secret, 48), seed));
    }

    acc = acc.wrapping_add(mix16_b(input.as_ptr(), secret.as_ptr(), seed));
    acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, input.len()-16), slice_offset_ptr(secret, 16), seed));

    avalanche(acc)
}

#[inline(always)]
fn xxh3_64_129to240(input: &[u8], seed: u64, secret: &[u8]) -> u64 {
    const START_OFFSET: usize = 3;
    const LAST_OFFSET: usize = 17;

    let mut acc = (input.len() as u64).wrapping_mul(xxh64::PRIME_1);
    let nb_rounds = input.len() / 16;

    for idx in 0..8 {
        acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, 16*idx), slice_offset_ptr(secret, 16*idx), seed));
    }
    acc = avalanche(acc);

    for idx in 8..nb_rounds {
        acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, 16*idx), slice_offset_ptr(secret, 16*(idx-8) + START_OFFSET), seed));
    }

    acc = acc.wrapping_add(mix16_b(slice_offset_ptr(input, input.len()-16), slice_offset_ptr(secret, SECRET_SIZE_MIN-LAST_OFFSET), seed));

    avalanche(acc)
}

fn xxh3_64_internal(input: &[u8], seed: u64, secret: &[u8], long_hash_fn: LongHashFn) -> u64 {
    debug_assert!(secret.len() >= SECRET_SIZE_MIN);

    if input.len() <= 16 {
        xxh3_64_0to16(input, seed, secret)
    } else if input.len() <= 128 {
        xxh3_64_7to128(input, seed, secret)
    } else if input.len() <= MID_SIZE_MAX {
        xxh3_64_129to240(input, seed, secret)
    } else {
        long_hash_fn(input, seed, secret)
    }
}

#[inline]
fn xxh3_64_long_impl(input: &[u8], secret: &[u8]) -> u64 {
    let mut acc = INITIAL_ACC;

    hash_long_internal_loop(&mut acc, input, secret);

    merge_accs(&mut acc, slice_offset_ptr(secret, SECRET_MERGEACCS_START), (input.len() as u64).wrapping_mul(xxh64::PRIME_1))
}

#[inline(never)]
fn xxh3_64_long_with_seed(input: &[u8], seed: u64, _secret: &[u8]) -> u64 {
    match seed {
        0 => xxh3_64_long_impl(input, &DEFAULT_SECRET),
        seed => xxh3_64_long_impl(input, &custom_default_secret(seed)),
    }
}

#[inline(never)]
fn xxh3_64_long_default(input: &[u8], _seed: u64, _secret: &[u8]) -> u64 {
    xxh3_64_long_impl(input, &DEFAULT_SECRET)
}


#[inline(never)]
fn xxh3_64_long_with_secret(input: &[u8], _seed: u64, secret: &[u8]) -> u64 {
    xxh3_64_long_impl(input, secret)
}

#[inline]
///Returns 64bit hash for provided input.
pub fn xxh3_64(input: &[u8]) -> u64 {
    xxh3_64_internal(input, 0, &DEFAULT_SECRET, xxh3_64_long_default)
}

#[inline]
///Returns 64bit hash for provided input using seed.
pub fn xxh3_64_with_seed(input: &[u8], seed: u64) -> u64 {
    xxh3_64_internal(input, seed, &DEFAULT_SECRET, xxh3_64_long_with_seed)
}

#[inline]
///Returns 64bit hash for provided input using custom secret.
pub fn xxh3_64_with_secret(input: &[u8], secret: &[u8]) -> u64 {
    xxh3_64_internal(input, 0, secret, xxh3_64_long_with_secret)
}

const INTERNAL_BUFFER_SIZE: usize = 256;

#[derive(Clone)]
#[repr(align(64))]
struct Aligned64<T>(T);

#[derive(Clone)]
///XXH3 Streaming algorithm
pub struct Xxh3 {
    acc: Acc,
    custom_secret: Aligned64<[u8; DEFAULT_SECRET_SIZE]>,
    buffer: Aligned64<[u8; INTERNAL_BUFFER_SIZE]>,
    buffered_size: u8,
    nb_stripes_acc: usize,
    total_len: u64,
    seed: u64,
}

impl Xxh3 {
    #[inline(always)]
    ///Creates new hasher with default settings
    pub const fn new() -> Self {
        Self::with_custom_ops(0, DEFAULT_SECRET)
    }

    #[inline]
    ///Creates new hasher with all options.
    const fn with_custom_ops(seed: u64, secret: [u8; DEFAULT_SECRET_SIZE]) -> Self {
        Self {
            acc: INITIAL_ACC,
            custom_secret: Aligned64(secret),
            buffer: Aligned64([0; INTERNAL_BUFFER_SIZE]),
            buffered_size: 0,
            nb_stripes_acc: 0,
            total_len: 0,
            seed,
        }
    }

    #[inline(always)]
    ///Creates new hasher with custom seed.
    pub const fn with_secret(secret: [u8; DEFAULT_SECRET_SIZE]) -> Self {
        Self::with_custom_ops(0, secret)
    }

    #[inline(always)]
    ///Creates new hasher with custom seed.
    pub const fn with_seed(seed: u64) -> Self {
        Self::with_custom_ops(seed, const_custom_default_secret(seed))
    }

    #[inline(always)]
    ///Resets state
    pub fn reset(&mut self) {
        self.acc = INITIAL_ACC;
        self.total_len = 0;
        self.buffered_size = 0;
        self.nb_stripes_acc = 0;
    }

    #[inline(always)]
    //We limit hashing variant to secrets with default size.
    const fn stripes_per_block() -> usize {
        DEFAULT_SECRET_LIMIT / SECRET_CONSUME_RATE
    }

    const fn internal_buffer_stripes() -> usize {
        INTERNAL_BUFFER_SIZE / STRIPE_LEN
    }

    #[inline]
    fn consume_stripes(acc: &mut Acc, nb_stripes: usize, nb_stripes_acc: usize, input: *const u8, secret: &[u8; DEFAULT_SECRET_SIZE]) -> usize {
        if (Self::stripes_per_block() - nb_stripes_acc) < nb_stripes {
            let stripes_to_end = Self::stripes_per_block() - nb_stripes_acc;
            let stripes_after_end = nb_stripes - stripes_to_end;

            accumulate_loop(acc, input, slice_offset_ptr(secret, nb_stripes_acc * SECRET_CONSUME_RATE), stripes_to_end);
            scramble_acc(acc, slice_offset_ptr(secret, DEFAULT_SECRET_LIMIT));
            accumulate_loop(acc, unsafe { input.add(stripes_to_end * STRIPE_LEN) }, secret.as_ptr(), stripes_after_end);
            stripes_to_end
        } else {
            accumulate_loop(acc, input, slice_offset_ptr(secret, nb_stripes_acc * SECRET_CONSUME_RATE), nb_stripes);
            nb_stripes_acc.wrapping_add(nb_stripes)
        }
    }

    ///Hashes provided chunk
    pub fn update(&mut self, mut input: &[u8]) {
        self.total_len = self.total_len.wrapping_add(input.len() as u64);

        if (input.len() + self.buffered_size as usize) <= INTERNAL_BUFFER_SIZE {
            unsafe {
                ptr::copy_nonoverlapping(input.as_ptr(), (self.buffer.0.as_mut_ptr() as *mut u8).offset(self.buffered_size as isize), input.len())
            }
            self.buffered_size += input.len() as u8;
            return;
        }

        if self.buffered_size > 0 {
            let fill_len = INTERNAL_BUFFER_SIZE - self.buffered_size as usize;

            unsafe {
                ptr::copy_nonoverlapping(input.as_ptr(), (self.buffer.0.as_mut_ptr() as *mut u8).offset(self.buffered_size as isize), fill_len)
            }

            self.nb_stripes_acc = Self::consume_stripes(&mut self.acc, Self::internal_buffer_stripes(), self.nb_stripes_acc, self.buffer.0.as_ptr(), &self.custom_secret.0);

            input = &input[fill_len..];
            self.buffered_size = 0;

        }

        if input.len() > INTERNAL_BUFFER_SIZE {
            loop {
                self.nb_stripes_acc = Self::consume_stripes(&mut self.acc, Self::internal_buffer_stripes(), self.nb_stripes_acc, input.as_ptr(), &self.custom_secret.0);
                input = &input[INTERNAL_BUFFER_SIZE..];

                if input.len() < INTERNAL_BUFFER_SIZE {
                    break;
                }
            }
        }

        unsafe {
            ptr::copy_nonoverlapping(input.as_ptr(), self.buffer.0.as_mut_ptr() as *mut u8, input.len())
        }
        self.buffered_size += input.len() as u8;
    }

    #[inline]
    fn digest_internal(&self, acc: &mut Acc) {
        if self.buffered_size as usize >= STRIPE_LEN {
            let nb_stripes = (self.buffered_size as usize - 1) / STRIPE_LEN;
            Self::consume_stripes(acc, nb_stripes, self.nb_stripes_acc, self.buffer.0.as_ptr(), &self.custom_secret.0);

            accumulate_512(acc, slice_offset_ptr(&self.buffer.0, self.buffered_size as usize - STRIPE_LEN), self.custom_secret.0.as_ptr());
        } else {
            let mut last_stripe = mem::MaybeUninit::<[u8; STRIPE_LEN]>::uninit();
            let catchup_size = STRIPE_LEN - self.buffered_size as usize;

            unsafe {
                ptr::copy_nonoverlapping(slice_offset_ptr(&self.buffer.0, self.buffer.0.len() - catchup_size), last_stripe.as_mut_ptr() as _, catchup_size);
                ptr::copy_nonoverlapping(self.buffer.0.as_ptr(), (last_stripe.as_mut_ptr() as *mut u8).add(catchup_size), self.buffered_size as usize);
            }

            accumulate_512(acc, last_stripe.as_ptr() as _, slice_offset_ptr(&self.custom_secret.0, self.custom_secret.0.len() - SECRET_LASTACC_START));
        }
    }

    ///Computes hash.
    pub fn digest(&self) -> u64 {
        if self.total_len > MID_SIZE_MAX as u64 {
            let mut acc = self.acc.clone();
            self.digest_internal(&mut acc);

            merge_accs(&mut acc, slice_offset_ptr(&self.custom_secret.0, SECRET_MERGEACCS_START), self.total_len.wrapping_mul(xxh64::PRIME_1))
        } else if self.seed > 0 {
            xxh3_64_with_seed(&self.buffer.0[..self.buffered_size as usize], self.seed)
        } else {
            xxh3_64_with_secret(&self.buffer.0[..self.buffered_size as usize], &self.custom_secret.0)
        }
    }
}
