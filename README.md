# xxhash-rust

![Rust](https://github.com/DoumanAsh/xxhash-rust/workflows/Rust/badge.svg?branch=master)
[![Crates.io](https://img.shields.io/crates/v/xxhash-rust.svg)](https://crates.io/crates/xxhash-rust)
[![Documentation](https://docs.rs/xxhash-rust/badge.svg)](https://docs.rs/crate/xxhash-rust/)

Implementation of [xxHash](https://github.com/Cyan4973/xxHash) in Rust

Version corresponds to xxHash [releases](https://github.com/Cyan4973/xxHash/releases)

Each algorithm is implemented via feature, allowing precise control over code size.

## Features:

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

## Version note

- Crate `0.8.0` contains invalid API and hence new increment was required.

- Crate `0.8.1` contains mistake in xxh3 algorithm, resulting in invalid input at input length equal to multiple of internal buffer + 1.
In addition to that when total length reaches 1025 and more

- As `0.8.0` and `0.8.1` are yanked, I consider it non-existing and hence `0.8.2` is the only version with stable API

In order to  keep up with original implementation version I'm not planning to bump major/minor until C implementation does so.
