//!Rust implementation of xxhash.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#[cfg(any(feature = "xxh32", feature = "const_xxh32"))]
mod xxh32_common;
#[cfg(feature = "xxh32")]
pub mod xxh32;
#[cfg(feature = "const_xxh32")]
pub mod const_xxh32;

#[cfg(any(feature = "xxh64", feature = "const_xxh64"))]
mod xxh64_common;
#[cfg(feature = "xxh64")]
pub mod xxh64;
#[cfg(feature = "const_xxh64")]
pub mod const_xxh64;
