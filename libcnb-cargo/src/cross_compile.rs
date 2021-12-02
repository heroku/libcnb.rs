use std::ffi::OsString;
use which::which;

/// Provides assistance for cross-compiling from the user's host platform to the desired target platform.
///
/// This function will not install required toolchains, linkers or compilers automatically. It will
/// look for the required tools and returns a human-readable help text if they cannot be found or
/// any other issue has been detected.
pub fn cross_compile_assistance(target_triple: impl AsRef<str>) -> CrossCompileAssistance {
    // Background: https://omarkhawaja.com/cross-compiling-rust-from-macos-to-linux/
    if target_triple.as_ref() == X86_64_UNKNOWN_LINUX_MUSL && cfg!(target_os = "macos") {
        let gcc_binary_name = "x86_64-linux-musl-gcc";

        match which(gcc_binary_name) {
            Ok(gcc_binary_path) => {
                CrossCompileAssistance::Configuration {
                    cargo_env: vec![
                        (
                            // Required until Cargo can auto-detect the musl-cross gcc/linker itself,
                            // since otherwise it checks for a binary named 'musl-gcc'
                            // not 'x86_64-linux-musl-gcc':
                            // https://github.com/FiloSottile/homebrew-musl-cross/issues/16
                            // https://github.com/rust-lang/cargo/issues/4133
                            OsString::from("CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"),
                            OsString::from(&gcc_binary_path),
                        ),
                        (
                            // Required so that any crates that call out to gcc are also cross-compiled:
                            // https://github.com/alexcrichton/cc-rs/issues/82
                            OsString::from("CC_x86_64_unknown_linux_musl"),
                            OsString::from(&gcc_binary_path),
                        ),
                    ],
                }
            }
            Err(_) => CrossCompileAssistance::HelpText(String::from(
                r#"For cross-compilation from macOS to x86_64-unknown-linux-musl, a C compiler and
linker for the target platform must be installed on your computer.

The easiest way to install 'x86_64-linux-musl-gcc' is to follow the instructions in the linked
GitHub repository:

https://github.com/FiloSottile/homebrew-musl-cross"#,
            )),
        }
    } else if target_triple.as_ref() == X86_64_UNKNOWN_LINUX_MUSL && cfg!(target_os = "linux") {
        match which("musl-gcc") {
            Ok(_) => CrossCompileAssistance::Configuration { cargo_env: vec![] },
            Err(_) => CrossCompileAssistance::HelpText(String::from(
                r#"For cross-compilation from Linux to x86_64-unknown-linux-musl, a C compiler and
linker for the target platform must be installed on your computer.

The easiest way to install 'musl-gcc' is to install the 'musl-tools' package:
- https://packages.ubuntu.com/focal/musl-tools
- https://packages.debian.org/bullseye/musl-tools"#,
            )),
        }
    } else {
        CrossCompileAssistance::NoAssistance
    }
}

pub enum CrossCompileAssistance {
    /// No specific assistance available for the current host and target platform combination.
    NoAssistance,
    /// A human-readable help text with instructions on how to setup the
    /// host machine for cross-compilation.
    HelpText(String),
    /// Required configuration to cross-compile to the target platoform.
    Configuration {
        cargo_env: Vec<(OsString, OsString)>,
    },
}

const X86_64_UNKNOWN_LINUX_MUSL: &str = "x86_64-unknown-linux-musl";
