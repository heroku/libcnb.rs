use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

    /// Creates a new buildpack from template
    Init(InitArgs),
}

#[derive(clap::ValueEnum, Clone, Eq, PartialEq, Hash, Debug)]
#[clap(rename_all = "snake_case")]
pub(crate) enum CodeOfConduct {
    Salesforce,
    ContributorCovenant,
}

#[derive(clap::ValueEnum, Clone, Eq, PartialEq, Hash, Debug)]
#[clap(rename_all = "lower")]
pub(crate) enum License {
    Mit,
    Bsd3,
}

#[derive(Parser, Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct InitArgs {
    /// Buildpack path
    pub destination: PathBuf,

    /// Buildpack name with namespace, must include a slash i.e. `heroku/ruby`
    #[arg(long = "name", default_value = "todo-namespace/todo-name")]
    pub name_namespace: String,

    /// Filename in the project's root used to detect if the buildpack will execute or not
    #[arg(long, default_value = "README.md")]
    pub detect_file: String,

    /// Generated license for the project
    #[arg(long, default_value = "mit")]
    pub license: License,

    /// Name of copyright holder for the license
    #[arg(long, default_value = "<TODO license holder name>")]
    pub copyright: String,

    /// Defines the code of conduct used in the generated project
    #[arg(long = "coc", default_value = "contributor_covenant")]
    pub conduct: CodeOfConduct,
}

impl InitArgs {
    pub fn name(&self) -> String {
        let mut parts = self.name_namespace.split('/');
        let _ = parts
            .next()
            .expect("Your provided name must contain a slash /");
        let name = parts
            .next()
            .expect("Your name included a slash, but nothing after it");

        name.to_string()
    }

    pub fn namespace(&self) -> String {
        let mut parts = self.name_namespace.split('/');
        let namespace = parts
            .next()
            .expect("Your provided name must contain a slash /");
        namespace.to_string()
    }
}

#[derive(Parser)]
pub(crate) struct PackageArgs {
    /// Disable cross-compile assistance
    #[arg(long)]
    pub no_cross_compile_assistance: bool,
    /// Build in release mode, with optimizations
    #[arg(long)]
    pub release: bool,
    /// Build for the target triple
    #[arg(long, default_value = "x86_64-unknown-linux-musl")]
    pub target: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, ValueEnum};

    #[test]
    fn verify_command() {
        // Trigger Clap's internal assertions that validate the command configuration.
        Cli::command().debug_assert();
    }

    #[test]
    fn test_enum_value() {
        assert_eq!(
            vec!["mit", "bsd3"],
            License::value_variants()
                .iter()
                .map(|var| var.to_possible_value().unwrap().get_name().to_string())
                .collect::<Vec<_>>()
        );
    }
}
