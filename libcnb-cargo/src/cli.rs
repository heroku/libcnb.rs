use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(bin_name = "cargo")]
pub(crate) enum Cli {
    #[command(subcommand)]
    Libcnb(LibcnbSubcommand),
}

#[derive(Subcommand)]
#[command(version, about, long_about = None)]
pub(crate) enum LibcnbSubcommand {
    /// Packages a libcnb.rs Cargo project as a Cloud Native Buildpack
    Package(PackageArgs),
}

#[derive(Parser)]
pub(crate) struct PackageArgs {
    /// Disable cross-compile assistance
    #[arg(long)]
    pub(crate) no_cross_compile_assistance: bool,
    /// Build in release mode, with optimizations
    #[arg(long)]
    pub(crate) release: bool,
    /// Build for the target triple
    #[arg(long)]
    pub(crate) target: Option<String>,
    /// Directory for packaged buildpacks, defaults to 'packaged' in Cargo workspace root
    #[arg(long)]
    pub(crate) package_dir: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_command() {
        // Trigger Clap's internal assertions that validate the command configuration.
        Cli::command().debug_assert();
    }
}
