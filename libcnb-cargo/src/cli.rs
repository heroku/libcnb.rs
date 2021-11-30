use clap::{App, AppSettings, Arg, SubCommand};

pub(crate) fn setup_cli_parsing<'a, 'b>() -> clap::App<'a, 'b> {
    App::new(env!("CARGO_PKG_NAME"))
        .bin_name("cargo")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("libcnb")
                .about("Allows working with buildpacks written with libcnb.rs")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("package")
                        .about("Packages a libcnb.rs Cargo project as a Cloud Native Buildpack")
                        .arg(
                            Arg::with_name("release")
                                .long("release")
                                .help("Build in release mode, with optimizations"),
                        )
                        .arg(
                            Arg::with_name("target")
                                .long("target")
                                .default_value("x86_64-unknown-linux-musl")
                                .help("Build for the target triple"),
                        )
                        .arg(
                            Arg::with_name("output-path")
                                .long("output")
                                .short("o")
                                .help("Write buildpack tarball to this path instead of Cargo's target directory")
                                .takes_value(true),
                        ),
                ),
        )
}
