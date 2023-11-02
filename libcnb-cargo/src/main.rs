#![doc = include_str!("../README.md")]
#![warn(unused_crate_dependencies)]
#![warn(clippy::pedantic)]
#![warn(clippy::panic_in_result_fn)]
#![warn(clippy::unwrap_used)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use libcnb_common as _;
#[cfg(test)]
use tempfile as _;

mod cli;
mod package;

use crate::cli::{Cli, LibcnbSubcommand};
use clap::Parser;

const UNSPECIFIED_ERROR: i32 = 1;

fn main() {
    match Cli::parse() {
        Cli::Libcnb(LibcnbSubcommand::Package(args)) => {
            if let Err(error) = package::execute(&args) {
                eprintln!("❌ {error}");
                std::process::exit(UNSPECIFIED_ERROR);
            }
        }
    }
}
