//!XXH3 implementation
//!
//!Provides only 64bit variant as most usable one.

use core::{ptr, mem};

use crate::xxh32_common as xxh32;
use crate::xxh64_common as xxh64;
use crate::xxh3_common::*;
pub use crate::xxh3_utils::*;

// Code is as close to original C implementation as possible
// It does make it look ugly, but it is fast and easy to update once xxhash gets new version.

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

#[inline(never)]
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

fn xxh3_64_long_with_seed(input: &[u8], seed: u64, _secret: &[u8]) -> u64 {
    match seed {
        0 => xxh3_64_long_impl(input, &DEFAULT_SECRET),
        seed => xxh3_64_long_impl(input, &custom_default_secret(seed)),
    }
}

fn xxh3_64_long_default(input: &[u8], _seed: u64, _secret: &[u8]) -> u64 {
    xxh3_64_long_impl(input, &DEFAULT_SECRET)
}

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
///
///Note: While overhead of deriving new secret from provided seed is low,
///it would more efficient to generate secret at compile time using special function
///`const_custom_default_secret`
pub fn xxh3_64_with_seed(input: &[u8], seed: u64) -> u64 {
    xxh3_64_internal(input, seed, &DEFAULT_SECRET, xxh3_64_long_with_seed)
}

#[inline]
///Returns 64bit hash for provided input using custom secret.
pub fn xxh3_64_with_secret(input: &[u8], secret: &[u8]) -> u64 {
    debug_assert!(secret.len() >= SECRET_SIZE_MIN);
    xxh3_64_internal(input, 0, secret, xxh3_64_long_with_secret)
}

const INTERNAL_BUFFER_SIZE: usize = 256;

#[derive(Clone)]
#[repr(align(64))]
struct Aligned64<T>(T);

#[derive(Clone)]
///XXH3 Streaming algorithm
///
///Internal state uses rather large buffers, therefore it might be beneficial
///to store hasher on heap rather than stack.
///Implementation makes no attempts at that, leaving choice entirely to user.
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

    #[inline]
    fn consume_stripes(acc: &mut Acc, nb_stripes: usize, nb_stripes_acc: usize, input: *const u8, secret: &[u8; DEFAULT_SECRET_SIZE]) -> usize {
        const STRIPES_PER_BLOCK: usize = (DEFAULT_SECRET_SIZE - STRIPE_LEN) / SECRET_CONSUME_RATE;

        if (STRIPES_PER_BLOCK - nb_stripes_acc) < nb_stripes {
            let stripes_to_end = STRIPES_PER_BLOCK - nb_stripes_acc;
            let stripes_after_end = nb_stripes - stripes_to_end;

            accumulate_loop(acc, input, slice_offset_ptr(secret, nb_stripes_acc * SECRET_CONSUME_RATE), stripes_to_end);
            scramble_acc(acc, slice_offset_ptr(secret, DEFAULT_SECRET_SIZE - STRIPE_LEN));
            accumulate_loop(acc, unsafe { input.add(stripes_to_end * STRIPE_LEN) }, secret.as_ptr(), stripes_after_end);
            stripes_to_end
        } else {
            accumulate_loop(acc, input, slice_offset_ptr(secret, nb_stripes_acc * SECRET_CONSUME_RATE), nb_stripes);
            nb_stripes_acc.wrapping_add(nb_stripes)
        }
    }

    ///Hashes provided chunk
    pub fn update(&mut self, mut input: &[u8]) {
        const INTERNAL_BUFFER_STRIPES: usize = INTERNAL_BUFFER_SIZE / STRIPE_LEN;

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

            self.nb_stripes_acc = Self::consume_stripes(&mut self.acc, INTERNAL_BUFFER_STRIPES, self.nb_stripes_acc, self.buffer.0.as_ptr(), &self.custom_secret.0);

            input = &input[fill_len..];
            self.buffered_size = 0;
        }

        if input.len() > INTERNAL_BUFFER_SIZE {
            loop {
                self.nb_stripes_acc = Self::consume_stripes(&mut self.acc, INTERNAL_BUFFER_STRIPES, self.nb_stripes_acc, input.as_ptr(), &self.custom_secret.0);
                input = &input[INTERNAL_BUFFER_SIZE..];

                if input.len() < INTERNAL_BUFFER_SIZE {
                    break;
                }
            }

            unsafe {
                ptr::copy_nonoverlapping(input.as_ptr().offset(-(STRIPE_LEN as isize)), (self.buffer.0.as_mut_ptr() as *mut u8).add(self.buffer.0.len() - STRIPE_LEN), STRIPE_LEN)
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

            accumulate_512(acc,
                           slice_offset_ptr(&self.buffer.0, self.buffered_size as usize - STRIPE_LEN),
                           slice_offset_ptr(&self.custom_secret.0, self.custom_secret.0.len() - STRIPE_LEN - SECRET_LASTACC_START)
            );
        } else {
            let mut last_stripe = mem::MaybeUninit::<[u8; STRIPE_LEN]>::uninit();
            let catchup_size = STRIPE_LEN - self.buffered_size as usize;
            debug_assert!(self.buffered_size > 0);

            unsafe {
                ptr::copy_nonoverlapping(slice_offset_ptr(&self.buffer.0, self.buffer.0.len() - catchup_size), last_stripe.as_mut_ptr() as _, catchup_size);
                ptr::copy_nonoverlapping(self.buffer.0.as_ptr(), (last_stripe.as_mut_ptr() as *mut u8).add(catchup_size), self.buffered_size as usize);
            }

            accumulate_512(acc, last_stripe.as_ptr() as _, slice_offset_ptr(&self.custom_secret.0, self.custom_secret.0.len() - STRIPE_LEN - SECRET_LASTACC_START));
        }
    }

    ///Computes hash.
    pub fn digest(&self) -> u64 {
        if self.total_len > MID_SIZE_MAX as u64 {
            let mut acc = self.acc.clone();
            self.digest_internal(&mut acc);

            merge_accs(&mut acc, slice_offset_ptr(&self.custom_secret.0, SECRET_MERGEACCS_START), self.total_len.wrapping_mul(xxh64::PRIME_1))
        } else if self.seed > 0 {
            //Technically we should not need to use it.
            //But in all actuality original xxh3 implementation uses default secret for input with size less or equal to MID_SIZE_MAX
            xxh3_64_internal(&self.buffer.0[..self.buffered_size as usize], self.seed, &DEFAULT_SECRET, xxh3_64_long_with_seed)
        } else {
            xxh3_64_internal(&self.buffer.0[..self.buffered_size as usize], self.seed, &self.custom_secret.0, xxh3_64_long_with_secret)
        }
    }
}

impl Default for Xxh3 {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl core::hash::Hasher for Xxh3 {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.digest()
    }

    #[inline(always)]
    fn write(&mut self, input: &[u8]) {
        self.update(input)
    }
}
