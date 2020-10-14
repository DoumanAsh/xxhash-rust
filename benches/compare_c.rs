use criterion::{criterion_group, criterion_main, Criterion};

const DATA: [&str; 34] = [
    "waifulandshigtgsqwetyuop[]asbnm,./",
    "waifulandshigtgsqwetyuop[]asbnm,.",
    "waifulandshigtgsqwetyuop[]asbnm,",
    "waifulandshigtgsqwetyuop[]asbnm",
    "waifulandshigtgsqwetyuop[]asbn",
    "waifulandshigtgsqwetyuop[]asb",
    "waifulandshigtgsqwetyuop[]as",
    "waifulandshigtgsqwetyuop[]a",
    "waifulandshigtgsqwetyuop[]",
    "waifulandshigtgsqwetyuop[",
    "waifulandshigtgsqwetyuop",
    "waifulandshigtgsqwetyuo",
    "waifulandshigtgsqwetyu",
    "waifulandshigtgsqwety",
    "waifulandshigtgsqwet",
    "waifulandshigtgsqwe",
    "waifulandshigtgsqw",
    "waifulandshigtgsq",
    "waifulandshigtgs",
    "waifulandshigtg",
    "waifulandshigt",
    "waifulandshig",
    "waifulandshi",
    "waifulandsh",
    "waifulands",
    "waifuland",
    "waifulan",
    "waifula",
    "waiful",
    "lolka",
    "lolk",
    "lol",
    "lo",
    "l",
];

fn define(c: &mut Criterion) {
    #[cfg(feature = "xxh32")]
    c.bench_function("xxh32 Rust", |b| b.iter_batched(|| &DATA, |data| for input in data {
        xxhash_rust::xxh32::xxh32(input.as_bytes(), 0);
    }, criterion::BatchSize::SmallInput));

    #[cfg(feature = "const_xxh32")]
    c.bench_function("const_xxh32 Rust", |b| b.iter_batched(|| &DATA, |data| for input in data {
        xxhash_rust::const_xxh32::xxh32(input.as_bytes(), 0);
    }, criterion::BatchSize::SmallInput));

    #[cfg(feature = "xxh32")]
    c.bench_function("xxh32 Rust Stateful", |b| b.iter_batched(|| &DATA, |data| for input in data {
        let mut hasher = xxhash_rust::xxh32::Xxh32::new(0);
        hasher.update(input.as_bytes());
        hasher.digest();
    }, criterion::BatchSize::SmallInput));

    #[cfg(feature = "xxh64")]
    c.bench_function("xxh64 Rust", |b| b.iter_batched(|| &DATA, |data| for input in data {
        xxhash_rust::xxh64::xxh64(input.as_bytes(), 0);
    }, criterion::BatchSize::SmallInput));

    #[cfg(feature = "xxh64")]
    c.bench_function("xxh64 Rust Stateful", |b| b.iter_batched(|| &DATA, |data| for input in data {
        let mut hasher = xxhash_rust::xxh64::Xxh64::new(0);
        hasher.update(input.as_bytes());
        hasher.digest();
    }, criterion::BatchSize::SmallInput));

    c.bench_function("twox-hash32 Rust", |b| b.iter_batched(|| &DATA, |data| for input in data {
        use core::hash::Hasher;

        let mut hasher = twox_hash::XxHash32::with_seed(0);
        hasher.write(input.as_bytes());
        hasher.finish();
    }, criterion::BatchSize::SmallInput));

    c.bench_function("twox-hash64 Rust", |b| b.iter_batched(|| &DATA, |data| for input in data {
        use core::hash::Hasher;

        let mut hasher = twox_hash::XxHash64::with_seed(0);
        hasher.write(input.as_bytes());
        hasher.finish();
    }, criterion::BatchSize::SmallInput));

    c.bench_function("xxh32 C", |b| b.iter_batched(|| &DATA, |data| for input in data {
        unsafe {
            xxhash_c_sys::XXH32(input.as_ptr() as _, input.len(), 0);
        }
    }, criterion::BatchSize::SmallInput));

    c.bench_function("xxh32 C Stateful", |b| b.iter_batched(|| &DATA, |data| for input in data {
        use xxhash_c_sys as sys;

        let mut state = core::mem::MaybeUninit::<sys::XXH32_state_t>::uninit();

        unsafe {
            sys::XXH32_reset(state.as_mut_ptr(), 0);
            sys::XXH32_update(state.as_mut_ptr(), input.as_ptr() as _, input.len());
            sys::XXH32_digest(state.as_mut_ptr());
        }
    }, criterion::BatchSize::SmallInput));

    c.bench_function("xxh64 C", |b| b.iter_batched(|| &DATA, |data| for input in data {
        unsafe {
            xxhash_c_sys::XXH64(input.as_ptr() as _, input.len(), 0);
        }
    }, criterion::BatchSize::SmallInput));

    c.bench_function("xxh64 C Stateful", |b| b.iter_batched(|| &DATA, |data| for input in data {
        use xxhash_c_sys as sys;

        let mut state = core::mem::MaybeUninit::<sys::XXH64_state_t>::uninit();

        unsafe {
            sys::XXH64_reset(state.as_mut_ptr(), 0);
            sys::XXH64_update(state.as_mut_ptr(), input.as_ptr() as _, input.len());
            sys::XXH64_digest(state.as_mut_ptr());
        }
    }, criterion::BatchSize::SmallInput));
}

criterion_group!(benches, define);

criterion_main!(benches);
