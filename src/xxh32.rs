//!32 bit version of xxhash algorithm
//!
//!Written using C implementation as reference.

use core::{ptr, slice};

use crate::utils::{slice_chunks, slice_aligned_chunks};
use crate::xxh32_common::*;

fn finalize(mut input: u32, data: &[u8], is_aligned: bool) -> u32 {
    let remainder = {
        match is_aligned {
            true => {
                let (chunks, remainder) = slice_aligned_chunks::<u32>(data);
                for chunk in chunks {
                    input = input.wrapping_add(
                        chunk.to_le().wrapping_mul(PRIME_3)
                    );
                    input = input.rotate_left(17).wrapping_mul(PRIME_4);
                }

                remainder
            },
            false => {
                let (chunks, remainder) = slice_chunks::<4>(data);
                for chunk in chunks {
                    input = input.wrapping_add(
                        u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_le().wrapping_mul(PRIME_3)
                    );
                    input = input.rotate_left(17).wrapping_mul(PRIME_4);
                }

                remainder
            }
        }
    };

    for byte in remainder {
        input = input.wrapping_add((*byte as u32).wrapping_mul(PRIME_5));
        input = input.rotate_left(11).wrapping_mul(PRIME_1);
    }

    avalanche(input)
}

#[inline(always)]
const fn init_v(seed: u32) -> (u32, u32, u32, u32) {
    (
        seed.wrapping_add(PRIME_1).wrapping_add(PRIME_2),
        seed.wrapping_add(PRIME_2),
        seed,
        seed.wrapping_sub(PRIME_1),
    )
}

macro_rules! round_loop {
    ($input:ident => $($v:tt)+) => {
        for chunk in $input {
            $($v)+.0 = round($($v)+.0, u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_le());
            $($v)+.1 = round($($v)+.1, u32::from_ne_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]).to_le());
            $($v)+.2 = round($($v)+.2, u32::from_ne_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]).to_le());
            $($v)+.3 = round($($v)+.3, u32::from_ne_bytes([chunk[12], chunk[13], chunk[14], chunk[15]]).to_le());
        }
    }
}

///Returns hash for the provided input
pub fn xxh32(mut input: &[u8], seed: u32) -> u32 {
    let mut result = input.len() as u32;

    if input.len() >= CHUNK_SIZE {
        let mut v = init_v(seed);

        let (chunks, remainder) = slice_chunks::<CHUNK_SIZE>(input);

        round_loop!(chunks => v);
        input = remainder;

        result = result.wrapping_add(
            v.0.rotate_left(1).wrapping_add(
                v.1.rotate_left(7).wrapping_add(
                    v.2.rotate_left(12).wrapping_add(
                        v.3.rotate_left(18)
                    )
                )
            )
        );
    } else {
        result = result.wrapping_add(seed.wrapping_add(PRIME_5));
    }

    return finalize(result, input, false);
}

///XXH32 Streaming algorithm
#[derive(Clone)]
pub struct Xxh32 {
    total_len: u32,
    is_large_len: bool,
    v: (u32, u32, u32, u32),
    mem: [u32; 4],
    mem_size: u32,
}

impl Xxh32 {
    #[inline]
    ///Creates new hasher with specified seed.
    pub const fn new(seed: u32) -> Self {
        Self {
            total_len: 0,
            is_large_len: false,
            v: init_v(seed),
            mem: [0, 0, 0, 0],
            mem_size: 0,
        }
    }

    ///Hashes provided input.
    pub fn update(&mut self, mut input: &[u8]) {
        self.total_len = self.total_len.wrapping_add(input.len() as u32);
        self.is_large_len |= (input.len() as u32 >= CHUNK_SIZE as u32) | (self.total_len >= CHUNK_SIZE as u32);

        if (self.mem_size + input.len() as u32) < CHUNK_SIZE as u32 {
            unsafe {
                ptr::copy_nonoverlapping(input.as_ptr(), (self.mem.as_mut_ptr() as *mut u8).offset(self.mem_size as isize), input.len())
            }
            self.mem_size += input.len() as u32;
            return
        }

        if self.mem_size > 0 {
            //previous if can fail only when we do not have enough space in buffer for input.
            //hence fill_len >= input.len()
            let fill_len = CHUNK_SIZE - self.mem_size as usize;

            unsafe {
                ptr::copy_nonoverlapping(input.as_ptr(), (self.mem.as_mut_ptr() as *mut u8).offset(self.mem_size as isize), fill_len)
            }

            self.v.0 = round(self.v.0, self.mem[0].to_le());
            self.v.1 = round(self.v.1, self.mem[1].to_le());
            self.v.2 = round(self.v.2, self.mem[2].to_le());
            self.v.3 = round(self.v.3, self.mem[3].to_le());

            input = &input[fill_len..];
            self.mem_size = 0;
        }

        let (chunks, remainder) = slice_chunks::<CHUNK_SIZE>(input);
        round_loop!(chunks => self.v);

        if remainder.len() > 0 {
            unsafe {
                ptr::copy_nonoverlapping(remainder.as_ptr(), self.mem.as_mut_ptr() as *mut u8, remainder.len())
            }
            self.mem_size = remainder.len() as u32;
        }
    }

    ///Finalize hashing.
    pub fn digest(&self) -> u32 {
        let mut result = self.total_len;

        if self.is_large_len {
            result = result.wrapping_add(
                self.v.0.rotate_left(1).wrapping_add(
                    self.v.1.rotate_left(7).wrapping_add(
                        self.v.2.rotate_left(12).wrapping_add(
                            self.v.3.rotate_left(18)
                        )
                    )
                )
            );
        } else {
            result = result.wrapping_add(self.v.2.wrapping_add(PRIME_5));
        }

        let input = unsafe {
            slice::from_raw_parts(self.mem.as_ptr() as *const u8, self.mem_size as usize)
        };

        return finalize(result, input, true);
    }

    #[inline]
    ///Resets the state with specified seed.
    pub fn reset(&mut self, seed: u32) {
        self.total_len = 0;
        self.is_large_len = false;
        self.v = init_v(seed);
        self.mem_size = 0;
    }
}

impl Default for Xxh32 {
    #[inline(always)]
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(feature = "std")]
impl std::io::Write for Xxh32 {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
