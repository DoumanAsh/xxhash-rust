//!Implementation of [xxHash](https://github.com/Cyan4973/xxHash) in Rust
//!
//!Version corresponds to xxHash [releases](https://github.com/Cyan4973/xxHash/releases)
//!
//!Each algorithm is implemented via feature, allowing precise control over code size.
//!
//!## Features:
//!
//!- `xxh32` - Enables 32bit algorithm. Suitable for x86 targets
//!- `const_xxh32` - `const fn` version of `xxh32` algorithm
//!- `xxh64` - Enables 64 algorithm. Suitable for x86_64 targets
//!- `const_xxh64` - `const fn` version of `xxh64` algorithm
//!- `xxh3` - Enables `xxh3` family of algorithms, superior to `xxh32` and `xxh64` in terms of performance.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#[cfg(any(feature = "xxh32", feature = "const_xxh32", feature = "xxh3"))]
mod xxh32_common;
#[cfg(feature = "xxh32")]
pub mod xxh32;
#[cfg(feature = "const_xxh32")]
pub mod const_xxh32;

#[cfg(any(feature = "xxh64", feature = "const_xxh64", feature = "xxh3"))]
mod xxh64_common;
#[cfg(feature = "xxh64")]
pub mod xxh64;
#[cfg(feature = "const_xxh64")]
pub mod const_xxh64;

#[cfg(feature = "xxh3")]
pub mod xxh3;
