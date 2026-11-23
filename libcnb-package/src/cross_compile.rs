use indoc::{formatdoc, indoc};
use std::env::consts;
use std::ffi::OsString;
use which::which;

/// Provides assistance for cross-compiling from the user's host platform to the desired target platform.
///
/// This function will not install required toolchains, linkers or compilers automatically. It will
/// look for the required tools and returns a human-readable help text if they can't be found or
/// any other issue has been detected.
pub fn cross_compile_assistance(target_triple: impl AsRef<str>) -> CrossCompileAssistance {
    let target_triple = target_triple.as_ref();
    let (toolchain_prefix, help_text) = match (target_triple, consts::OS, consts::ARCH) {
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_X86_64) => (
            "aarch64-linux-gnu",
            indoc! {"
                To install an aarch64 cross-compiler on Ubuntu:
                sudo apt-get install g++-aarch64-linux-gnu libc6-dev-arm64-cross musl-tools
            "},
        ),
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_MACOS, ARCH_X86_64 | ARCH_AARCH64) => (
            "aarch64-unknown-linux-musl",
            indoc! {"
                To install an aarch64 cross-compiler on macOS:
                brew install messense/macos-cross-toolchains/aarch64-unknown-linux-musl
            "},
        ),
        // When the target matches the host architecture, Cargo will automatically select the
        // appropriate default linker and set the required environment variables. We only need
        // to verify that musl-gcc is available.
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_AARCH64)
        | (X86_64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_X86_64) => {
            return match which("musl-gcc") {
                Ok(_) => CrossCompileAssistance::Configuration {
                    cargo_env: Vec::new(),
                },
                Err(_) => CrossCompileAssistance::HelpText(formatdoc! {"
                    For cross-compilation from {0} {1} to {target_triple},
                    a C compiler and linker for the target platform must be installed:

                    To install musl-tools on Ubuntu:
                    sudo apt-get install musl-tools

                    You will also need to install the Rust target:
                    rustup target add {target_triple}
                    ",
                    consts::ARCH,
                    consts::OS
                }),
            };
        }
        (X86_64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_AARCH64) => (
            "x86_64-linux-gnu",
            indoc! {"
                To install an x86_64 cross-compiler on Ubuntu:
                sudo apt-get install g++-x86-64-linux-gnu libc6-dev-amd64-cross musl-tools
            "},
        ),
        (X86_64_UNKNOWN_LINUX_MUSL, OS_MACOS, ARCH_X86_64 | ARCH_AARCH64) => (
            "x86_64-unknown-linux-musl",
            indoc! {"
                To install an x86_64 cross-compiler on macOS:
                brew install messense/macos-cross-toolchains/x86_64-unknown-linux-musl
            "},
        ),
        _ => return CrossCompileAssistance::NoAssistance,
    };

    let gcc = format!("{toolchain_prefix}-gcc");
    let gxx = format!("{toolchain_prefix}-g++");
    let ar = format!("{toolchain_prefix}-ar");

    match which(&gcc) {
        Ok(_) => {
            let target_env_suffix = target_triple.replace('-', "_");
            CrossCompileAssistance::Configuration {
                cargo_env: vec![
                    (
                        // Required until Cargo can auto-detect the musl-cross gcc/linker itself,
                        // since otherwise it checks for a binary named 'musl-gcc' (which is handled above):
                        // https://github.com/rust-lang/cargo/issues/4133
                        OsString::from(format!(
                            "CARGO_TARGET_{}_LINKER",
                            target_env_suffix.to_uppercase()
                        )),
                        OsString::from(&gcc),
                    ),
                    (
                        // Required so that any crates that call out to gcc are also cross-compiled:
                        // https://github.com/alexcrichton/cc-rs/issues/82
                        OsString::from(format!("CC_{target_env_suffix}")),
                        OsString::from(&gcc),
                    ),
                    (
                        // Required so that crates using the `cc` crate for C++ compilation
                        // use the cross-compilation toolchain instead of the host toolchain:
                        // https://github.com/heroku/libcnb.rs/issues/725
                        OsString::from(format!("CXX_{target_env_suffix}")),
                        OsString::from(&gxx),
                    ),
                    (
                        // Required so that crates using the `cc` crate to create static
                        // libraries use the cross-compilation archiver:
                        // https://github.com/heroku/libcnb.rs/issues/725
                        OsString::from(format!("AR_{target_env_suffix}")),
                        OsString::from(ar),
                    ),
                ],
            }
        }
        Err(_) => CrossCompileAssistance::HelpText(formatdoc! {"
            For cross-compilation from {0} {1} to {target_triple},
            a C compiler and linker for the target platform must be installed:

            {help_text}
            You will also need to install the Rust target:
            rustup target add {target_triple}
            ",
            consts::ARCH,
            consts::OS
        }),
    }
}

pub enum CrossCompileAssistance {
    /// No specific assistance available for the current host and target platform combination.
    NoAssistance,
    /// A human-readable help text with instructions on how to setup the
    /// host machine for cross-compilation.
    HelpText(String),
    /// Required configuration to cross-compile to the target platform.
    Configuration {
        cargo_env: Vec<(OsString, OsString)>,
    },
}

// Constants for supported target triples
pub const AARCH64_UNKNOWN_LINUX_MUSL: &str = "aarch64-unknown-linux-musl";
pub const X86_64_UNKNOWN_LINUX_MUSL: &str = "x86_64-unknown-linux-musl";

// Constants for `std::env::consts::OS` and `std::env::consts::ARCH`
const OS_LINUX: &str = "linux";
const OS_MACOS: &str = "macos";
const ARCH_X86_64: &str = "x86_64";
const ARCH_AARCH64: &str = "aarch64";
