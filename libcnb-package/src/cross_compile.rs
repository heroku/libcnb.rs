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
    let (gcc_path, help_text) = match (target_triple, consts::OS, consts::ARCH) {
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_X86_64) => (
            "aarch64-linux-gnu-gcc",
            "To install an aarch64 cross-compiler on Ubuntu:\nsudo apt-get install g++-aarch64-linux-gnu",
        ),
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_MACOS, ARCH_X86_64 | ARCH_AARCH64) => (
            "aarch64-unknown-linux-musl-gcc",
            "To install an aarch64 cross-compiler on macOS:\nbrew install messense/macos-cross-toolchains/aarch64-unknown-linux-musl",
        ),
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_AARCH64) | (X86_64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_X86_64) => (
            "musl-gcc",
            "To install musl-tools on Ubuntu:\nsudo apt-get install musl-tools",
        ),
        (X86_64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_AARCH64) => (
            "x86_64-linux-gnu-gcc",
            "To install an x86_64 cross-compiler on Ubuntu:\nsudo apt-get install g++-x86_64-linux-gnu",
        ),
        (X86_64_UNKNOWN_LINUX_MUSL, OS_MACOS, ARCH_X86_64 | ARCH_AARCH64) => (
            "x86_64-unknown-linux-musl-gcc",
            "To install an x86_64 cross-compiler on macOS:\nbrew install messense/macos-cross-toolchains/x86_64-unknown-linux-musl",
        ),
        _ => return CrossCompileAssistance::NoAssistance,
    };

    match which(gcc_path) {
        Ok(_) => {
            if gcc_path == "musl-gcc" {
                CrossCompileAssistance::Configuration {
                    cargo_env: Vec::new(),
                }
            } else {
                CrossCompileAssistance::Configuration {
                    cargo_env: vec![
                        (
                            // Required until Cargo can auto-detect the musl-cross gcc/linker itself,
                            // since otherwise it checks for a binary named 'musl-gcc' (which is handled above):
                            // https://github.com/rust-lang/cargo/issues/4133
                            OsString::from(format!(
                                "CARGO_TARGET_{}_LINKER",
                                target_triple.to_uppercase().replace('-', "_")
                            )),
                            OsString::from(gcc_path),
                        ),
                        (
                            // Required so that any crates that call out to gcc are also cross-compiled:
                            // https://github.com/alexcrichton/cc-rs/issues/82
                            OsString::from(format!("CC_{}", target_triple.replace('-', "_"))),
                            OsString::from(gcc_path),
                        ),
                    ],
                }
            }
        }
        Err(_) => CrossCompileAssistance::HelpText(format!(
            r"For cross-compilation from {0} {1} to {target_triple}, a C compiler and
linker for the target platform must be installed:

{help_text}
            
You will also need to install the Rust target:
rustup target add {target_triple}",
            consts::ARCH,
            consts::OS
        )),
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
const AARCH64_UNKNOWN_LINUX_MUSL: &str = "aarch64-unknown-linux-musl";
const X86_64_UNKNOWN_LINUX_MUSL: &str = "x86_64-unknown-linux-musl";

// Constants for `std::env::consts::OS` and `std::env::consts::ARCH`
const OS_LINUX: &str = "linux";
const OS_MACOS: &str = "macos";
const ARCH_X86_64: &str = "x86_64";
const ARCH_AARCH64: &str = "aarch64";
