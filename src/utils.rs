//! Better byte working utilities

use core::{mem, slice, ptr};

///Splits slice into tuple as follows:
///
///- Reference to slice of concrete chunks by size N
///- Remainder slice whose size is < N
pub const fn slice_chunks<const N: usize>(input: &[u8]) -> (&[[u8; N]], &[u8]) {
    slice_aligned_chunks::<[u8; N]>(input)
}

///Splits slice into tuple as follows:
///
///- Reference to slice of concrete T by size N.
///- Remainder slice whose size is < N
///
///This function assumes input is aligned buffer, if that's not guaranteed, use `slice_chunks`
pub const fn slice_aligned_chunks<T: Copy>(input: &[u8]) -> (&[T], &[u8]) {
    debug_assert!(mem::size_of::<T>() > 0); //N MUST be positive
    let input_len = input.len();

    //First we need to split slice into two parts:
    //- Slice of chunks
    let chunks_len = input_len / mem::size_of::<T>();
    let split_at = chunks_len * mem::size_of::<T>();
    let chunks = unsafe {
        //We know exact size N so cast it immediately
        slice::from_raw_parts(input.as_ptr() as _, chunks_len)
    };
    //- Remainder
    let rest = unsafe {
        slice::from_raw_parts(input.as_ptr().add(split_at), input_len.saturating_sub(split_at))
    };

    (chunks, rest)
}

pub struct Buffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub offset: usize,
}

impl Buffer {
    #[inline(always)]
    pub const fn copy_from_slice(&self, src: &[u8]) {
        self.copy_from_slice_by_size(src, src.len())
    }

    #[inline(always)]
    pub const fn copy_from_slice_by_size(&self, src: &[u8], len: usize) {
        debug_assert!(self.len.saturating_sub(self.offset) >= len);

        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), self.ptr.add(self.offset), len);
        }
    }
}
