//!Rust implementation of xxhash.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#[cfg(feature = "xxh32")]
pub mod xxh32;
#[cfg(feature = "const_xxh32")]
pub mod const_xxh32;
