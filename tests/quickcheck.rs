extern crate quickcheck;
#[allow(unused_imports)]
#[macro_use]
extern crate quickcheck_macros;

#[cfg(any(feature = "xxh32", feature = "xxh64", feature = "xxh3"))]
mod tests {
    use quickcheck::TestResult;
    use std::hash::Hasher;
    use std::num::{NonZeroU8, NonZeroUsize};
    use xxhash_c_sys as sys;

    // In practice 2048 bytes of data should cover all cases for the streaming hashers.
    // So we use a limit 10 times that to cover more chunking variations.
    const MAX_STREAM_SIZE: usize = 2048 * 10;

    #[cfg(feature = "xxh3")]
    #[quickcheck]
    fn xxh3_chunked_matches_buffered(
        chunk_size: NonZeroUsize,
        xs: Vec<u8>,
        times: NonZeroU8,
        additional: u8,
    ) -> TestResult {
        // additional argument doubles down as the hasher seed
        let seed = additional as u64;
        // the vecs produced by quickcheck are perhaps a bit small by default.
        // additional should add some noise to avoid only getting nice even lengths.
        let target_size = (xs.len() * times.get() as usize + additional as usize) % MAX_STREAM_SIZE;
        let xs = xs.into_iter().cycle().take(target_size).collect::<Vec<_>>();

        // write all at once
        let mut h0 = xxhash_rust::xxh3::Xxh3::with_seed(seed);
        h0.write(&xs);
        let h0 = h0.finish();

        // write in chunks
        let mut h1 = xxhash_rust::xxh3::Xxh3::with_seed(seed);
        for chunk in xs.chunks(chunk_size.get()) {
            h1.write(chunk);
        }
        let h1 = h1.finish();

        let one_shot_result = xxhash_rust::xxh3::xxh3_64_with_seed(&xs, seed);

        let sys_result = unsafe { sys::XXH3_64bits_withSeed(xs.as_ptr() as _, xs.len(), seed) };

        // compare all against reference
        assert_eq!(h0, sys_result);
        assert_eq!(h1, sys_result);
        assert_eq!(one_shot_result, sys_result);

        TestResult::passed()
    }

    #[cfg(feature = "xxh64")]
    #[quickcheck]
    fn xxh64_chunked_matches_buffered(
        chunk_size: NonZeroUsize,
        xs: Vec<u8>,
        times: NonZeroU8,
        additional: u8,
    ) -> TestResult {
        // additional argument doubles down as the hasher seed
        let seed = additional as u64;
        // the vecs produced by quickcheck are perhaps a bit small by default.
        // additional should add some noise to avoid only getting nice even lengths.
        let target_size = (xs.len() * times.get() as usize + additional as usize) % MAX_STREAM_SIZE;
        let xs = xs.into_iter().cycle().take(target_size).collect::<Vec<_>>();

        // write all at once
        let mut h0 = xxhash_rust::xxh64::Xxh64::new(seed);
        h0.write(&xs);
        let h0 = h0.finish();

        // write in chunks
        let mut h1 = xxhash_rust::xxh64::Xxh64::new(seed);
        for chunk in xs.chunks(chunk_size.get()) {
            h1.write(chunk);
        }
        let h1 = h1.finish();

        let one_shot_result = xxhash_rust::xxh64::xxh64(&xs, seed);

        let sys_result = unsafe { sys::XXH64(xs.as_ptr() as _, xs.len(), seed) };

        // compare all against reference
        assert_eq!(h0, sys_result);
        assert_eq!(h1, sys_result);
        assert_eq!(one_shot_result, sys_result);

        TestResult::passed()
    }

    #[cfg(feature = "xxh32")]
    #[quickcheck]
    fn xxh32_chunked_matches_buffered(
        chunk_size: NonZeroUsize,
        xs: Vec<u8>,
        times: NonZeroU8,
        additional: u8,
    ) -> TestResult {
        // additional argument doubles down as the hasher seed
        let seed = additional as u32;
        // the vecs produced by quickcheck are perhaps a bit small by default.
        // additional should add some noise to avoid only getting nice even lengths.
        let target_size = (xs.len() * times.get() as usize + additional as usize) % MAX_STREAM_SIZE;
        let xs = xs.into_iter().cycle().take(target_size).collect::<Vec<_>>();

        // write all at once
        let mut h0 = xxhash_rust::xxh32::Xxh32::new(seed);
        h0.update(&xs);
        let h0 = h0.digest();

        // write in chunks
        let mut h1 = xxhash_rust::xxh32::Xxh32::new(seed);
        for chunk in xs.chunks(chunk_size.get()) {
            h1.update(chunk);
        }
        let h1 = h1.digest();

        let one_shot_result = xxhash_rust::xxh32::xxh32(&xs, seed);

        let sys_result = unsafe { sys::XXH32(xs.as_ptr() as _, xs.len(), seed) };

        // compare all against reference
        assert_eq!(h0, sys_result);
        assert_eq!(h1, sys_result);
        assert_eq!(one_shot_result, sys_result);

        TestResult::passed()
    }
}
