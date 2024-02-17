# xxhash-rust

![Rust](https://github.com/DoumanAsh/xxhash-rust/workflows/Rust/badge.svg?branch=master)
[![Crates.io](https://img.shields.io/crates/v/xxhash-rust.svg)](https://crates.io/crates/xxhash-rust)
[![Documentation](https://docs.rs/xxhash-rust/badge.svg)](https://docs.rs/crate/xxhash-rust/)

Implementation of [xxHash](https://github.com/Cyan4973/xxHash) in Rust

Each algorithm is implemented via feature, allowing precise control over code size.

## Example

- Cargo.toml

```toml
[dependencies.xxhash-rust]
version = "0.8.5"
features = ["xxh3", "const_xxh3"]
```

- main.rs

```rust
use xxhash_rust::const_xxh3::xxh3_64 as const_xxh3;
use xxhash_rust::xxh3::xxh3_64;

const TEST: u64 = const_xxh3(b"TEST");

fn test_input(text: &str) -> bool {
    match xxh3_64(text.as_bytes()) {
        TEST => true,
        _ => false
    }
}

assert!(!test_input("tEST"));
assert!(test_input("TEST"));
```

## Features:

By default all features are off.

- `xxh32` - Enables 32bit algorithm. Suitable for x86 targets
- `const_xxh32` - `const fn` version of `xxh32` algorithm
- `xxh64` - Enables 64 algorithm. Suitable for x86_64 targets
- `const_xxh64` - `const fn` version of `xxh64` algorithm
- `xxh3` - Enables `xxh3` family of algorithms, superior to `xxh32` and `xxh64` in terms of performance.
- `const_xxh3` - `const fn` version of `xxh3` algorithm

## HW acceleration

Similar to reference implementation, crate implements various SIMDs in `xxh3` depending on provided flags.
All checks are performed only at compile time, hence user is encouraged to enable these accelerations (for example via `-C target_cpu=native`)

Used SIMD acceleration:

- SSE2 - widely available, can be safely enabled in 99% of cases. Enabled by default in `x86_64` targets.
- AVX2;
- Neon - Enabled by default on aarch64 targets (most likely)
- Wasm SIMD128 - Has to be enabled via rust flag: `-Ctarget-feature=+simd128`

## Streaming vs One-shot

For performance reasons one-shot version of algorithm does not re-use streaming version.
Unless needed, user is advised to use one-shot version which tends to be more optimal.

## `const fn` version

While `const fn` provides compile time implementation, it does so at performance cost.
Hence you should only use it at _compile_ time.

To guarantee that something is computed at compile time make sure to initialize hash output
as `const` or `static` variable, otherwise it is possible function is executed at runtime, which
would be worse than regular algorithm.

`const fn` is implemented in best possible way while conforming to limitations of Rust `const
fn`, but these limitations are quite strict making any high performance code impossible.

## Version note

- `0.8.*` corresponds to C's `0.8.*`

In order to  keep up with original implementation version I'm not planning to bump major/minor until C implementation does so.

## Comparison with twox-hash

Refer to my [comment](https://github.com/DoumanAsh/xxhash-rust/issues/10#issuecomment-980488647)
