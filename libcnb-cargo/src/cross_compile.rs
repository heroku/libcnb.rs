use std::ffi::OsString;
use which::which;

/// Constructs a set of environment variables to enable cross-compiling from the user's host
/// platform to the desired target platform.
///
/// This function will not install required toolchains, linkers or compilers automatically. It will
/// look for the required tools and returns an error if they cannot be found. See
/// [`cross_compile_help`] on how to support users setting up their machine for cross-compilation.
///
/// # Errors
///
/// Will return `Err` if the cross-compile environment could not be created for the current host
/// and target platform.
pub fn cross_compile_env(
    target_triple: impl AsRef<str>,
) -> Result<Vec<(OsString, OsString)>, CrossCompileError> {
    if target_triple.as_ref() == "x86_64-unknown-linux-musl" && cfg!(target_os = "macos") {
        let gcc_binary_name = "x86_64-linux-musl-gcc";

        which(gcc_binary_name)
            .map(|gcc_binary_path| {
                vec![
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
                ]
            })
            .map_err(|_| {
                CrossCompileError::CouldNotFindRequiredBinary(String::from(gcc_binary_name))
            })
    } else {
        Ok(vec![])
    }
}

#[derive(Debug)]
pub enum CrossCompileError {
    CouldNotFindRequiredBinary(String),
}

/// Returns a human-readable help text about cross-compiling from the user's host platform to
/// the desired target platform.
pub fn cross_compile_help(target_triple: impl AsRef<str>) -> Option<String> {
    if target_triple.as_ref() == "x86_64-unknown-linux-musl" && cfg!(target_os = "macos") {
        Some(String::from(
            r#"For cross-compilation from macOS to x86_64-unknown-linux-musl, a C compiler and linker for the
target platform must be installed on your computer.

The easiest way to install 'x86_64-linux-musl-ld' and 'x86_64-linux-musl-gcc', is to follow the
instructions in the linked GitHub repository:

https://github.com/FiloSottile/homebrew-musl-cross"#,
        ))
    } else {
        None
    }
}
