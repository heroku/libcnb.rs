#![doc = include_str!("../README.md")]

#[cfg(feature = "command")]
pub mod command;
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
#[cfg(feature = "output")]
pub mod output;
#[cfg(feature = "tar")]
pub mod tar;
#[cfg(feature = "toml")]
pub mod toml;
#[cfg(feature = "write")]
pub mod write;
#[cfg(test)]
use fun_run as _;
