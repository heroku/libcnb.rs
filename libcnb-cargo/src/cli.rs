use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(bin_name = "cargo")]
pub(crate) enum Cli {
    #[clap(subcommand)]
    Libcnb(LibcnbSubcommand),
}

#[derive(Subcommand)]
#[clap(version, about, long_about = None)]
pub(crate) enum LibcnbSubcommand {
    /// Packages a libcnb.rs Cargo project as a Cloud Native Buildpack
    Package(PackageArgs),
}

#[derive(Parser)]
pub(crate) struct PackageArgs {
    /// Disable cross-compile assistance
    #[clap(long)]
    pub no_cross_compile_assistance: bool,
    /// Build in release mode, with optimizations
    #[clap(long)]
    pub release: bool,
    /// Build for the target triple
    #[clap(long, default_value = "x86_64-unknown-linux-musl")]
    pub target: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_command() {
        Cli::command().debug_assert();
    }
}
