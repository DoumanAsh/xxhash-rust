extern crate quickcheck;
#[allow(unused_imports)]
#[macro_use]
extern crate quickcheck_macros;

#[cfg(feature = "xxh3")]
mod tests {
    use quickcheck::TestResult;
    use std::hash::Hasher;
    use std::num::{NonZeroU8, NonZeroUsize};
    use xxhash_c_sys as sys;
    use xxhash_rust::xxh3::{xxh3_64, xxh3_64_with_seed, Xxh3};

    #[quickcheck]
    fn chunked_matches_buffered(
        xs: Vec<u8>,
        chunk_size: NonZeroUsize,
        times: NonZeroU8,
        additional: u8,
    ) -> TestResult {
        let target_size = xs.len() * times.get() as usize + additional as usize;
        if xs.is_empty() || target_size > 10_000_000 {
            TestResult::discard()
        } else {
            let xs = xs
                .into_iter()
                .cycle()
                // the vecs produced by quickcheck are perhaps a bit small by default.
                // additional should add some noise to avoid only getting nice even lengths.
                .take(target_size)
                .collect::<Vec<_>>();

            // write all at once
            let mut h0 = Xxh3::default();
            h0.write(&xs);
            let h0 = h0.finish();

            // write in chunks
            let mut h1 = Xxh3::default();
            for chunk in xs.chunks(chunk_size.get()) {
                h1.write(chunk);
            }
            let h1 = h1.finish();

            let sys_result = unsafe { sys::XXH3_64bits(xs.as_ptr() as _, xs.len()) };

            // compare all, including to buffered and reference
            let outcome = h0 == h1 && h0 == xxh3_64(&xs) && h0 == sys_result;

            TestResult::from_bool(outcome)
        }
    }

    #[quickcheck]
    fn chunked_matches_buffered_seed(
        seed: u64,
        xs: Vec<u8>,
        chunk_size: NonZeroUsize,
        times: NonZeroU8,
        additional: u8,
    ) -> TestResult {
        let target_size = xs.len() * times.get() as usize + additional as usize;
        if xs.is_empty() || target_size > 10_000_000 {
            TestResult::discard()
        } else {
            let xs = xs
                .into_iter()
                .cycle()
                // the vecs produced by quickcheck are perhaps a bit small by default.
                // additional should add some noise to avoid only getting nice even lengths.
                .take(target_size)
                .collect::<Vec<_>>();

            // write all at once
            let mut h0 = Xxh3::with_seed(seed);
            h0.write(&xs);
            let h0 = h0.finish();

            // write in chunks
            let mut h1 = Xxh3::with_seed(seed);
            for chunk in xs.chunks(chunk_size.get()) {
                h1.write(chunk);
            }
            let h1 = h1.finish();

            let sys_result = unsafe { sys::XXH3_64bits_withSeed(xs.as_ptr() as _, xs.len(), seed) };

            // compare all, including to buffered and reference
            let outcome = h0 == h1 && h0 == xxh3_64_with_seed(&xs, seed) && h0 == sys_result;

            TestResult::from_bool(outcome)
        }
    }

    #[quickcheck]
    fn short(xs: Vec<u8>) -> TestResult {
        if xs.is_empty() || xs.len() > 8 {
            TestResult::discard()
        } else {
            // write all at once
            let mut h0 = Xxh3::default();
            h0.write(&xs);
            let h0 = h0.finish();

            // write in chunks
            let mut h1 = Xxh3::default();
            for chunk in xs.chunks(1) {
                h1.write(chunk);
            }
            let h1 = h1.finish();

            let sys_result = unsafe { sys::XXH3_64bits(xs.as_ptr() as _, xs.len()) };

            // compare all, including to buffered and reference
            let outcome = h0 == h1 && h0 == xxh3_64(&xs) && h0 == sys_result;

            TestResult::from_bool(outcome)
        }
    }
}
