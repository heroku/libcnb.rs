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
        // There is more than just one binary name here since we also support the binary name for
        // an older version cross_compile_assistance which suggested installing a different
        // toolchain.
        let possible_gcc_binary_names =
            vec!["x86_64-unknown-linux-musl-gcc", "x86_64-linux-musl-gcc"];

        possible_gcc_binary_names
            .iter()
            .find_map(|binary_name| which(binary_name).ok())
            .map_or_else(|| CrossCompileAssistance::HelpText(String::from(
                r#"For cross-compilation from macOS to x86_64-unknown-linux-musl, a C compiler and
linker for the target platform must be installed on your computer.

The easiest way to install the required cross-compilation toolchain is to run:
brew install messense/macos-cross-toolchains/x86_64-unknown-linux-musl

For more information, see:
https://github.com/messense/homebrew-macos-cross-toolchains"#,
            )), |gcc_binary_path| {
                CrossCompileAssistance::Configuration {
                    cargo_env: vec![
                        (
                            // Required until Cargo can auto-detect the musl-cross gcc/linker itself,
                            // since otherwise it checks for a binary named 'musl-gcc':
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
            })
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
    /// Required configuration to cross-compile to the target platform.
    Configuration {
        cargo_env: Vec<(OsString, OsString)>,
    },
}

const X86_64_UNKNOWN_LINUX_MUSL: &str = "x86_64-unknown-linux-musl";
