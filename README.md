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
