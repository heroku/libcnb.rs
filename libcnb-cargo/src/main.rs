#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod cli;
mod logging;
mod package;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use fs_extra as _;
#[cfg(test)]
use tempfile as _;

use crate::cli::{Cli, LibcnbSubcommand};
use crate::package::run_package_command;
use clap::Parser;

fn main() {
    match Cli::parse() {
        Cli::Libcnb(LibcnbSubcommand::Package(args)) => run_package_command(&args),
    }
}
