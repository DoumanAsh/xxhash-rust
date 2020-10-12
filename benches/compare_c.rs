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

#[cfg(feature = "v32")]
fn v32_bench(c: &mut Criterion) {

    c.bench_function("V32 Rust", |b| b.iter_batched(|| &DATA, |data| for input in data {
        xxhash_rust::v32::xxh32(input.as_bytes(), 0);
    }, criterion::BatchSize::SmallInput));

    c.bench_function("V32 Rust Stateful", |b| b.iter_batched(|| &DATA, |data| for input in data {
        let mut hasher = xxhash_rust::v32::Xxh32::new(0);
        hasher.update(input.as_bytes());
        hasher.finish();
    }, criterion::BatchSize::SmallInput));

    c.bench_function("V32 C", |b| b.iter_batched(|| &DATA, |data| for input in data {
        unsafe {
            xxhash_c_sys::XXH32(input.as_ptr() as _, input.len(), 0);
        }
    }, criterion::BatchSize::SmallInput));
}

#[cfg(feature = "v32")]
criterion_group!(benches, v32_bench);

criterion_main!(benches);
