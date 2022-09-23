#![doc = include_str!("../README.md")]
// Enable lints that are disabled by default.
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
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
