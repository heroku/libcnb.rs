//! Opinionated common code for buildpacks implemented with libcnb.rs
//!
//! Contains common helpers and functionality that is not present in the more generic libcnb.rs library.

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// In most cases adding error docs provides little value.
#![allow(clippy::missing_errors_doc)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "digest")]
pub mod digest;
#[cfg(feature = "download")]
pub mod download;
#[cfg(feature = "error")]
pub mod error;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "log")]
pub mod log;
#[cfg(feature = "tar")]
pub mod tar;
#[cfg(feature = "toml")]
pub mod toml;
