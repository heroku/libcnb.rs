use std::ffi::OsString;
use which::which;

/// Constructs a set of environment variables to enable cross-compiling from the user's host
/// platform to the desired target platform.
///
/// This function will not install required toolchains, linkers or compilers automatically. It will
/// look for the required tools and returns an error if they cannot be found. See
/// [`cross_compile_help`] on how to support users setting up their machine for cross-compilation.
pub fn cross_compile_env(
    target_triple: impl AsRef<str>,
) -> Result<Vec<(OsString, OsString)>, CrossCompileError> {
    if target_triple.as_ref() == "x86_64-unknown-linux-musl" && cfg!(target_os = "macos") {
        let ld_binary_name = "x86_64-linux-musl-ld";
        let cc_binary_name = "x86_64-linux-musl-gcc";

        Ok(vec![
            (
                OsString::from("CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"),
                which(ld_binary_name)
                    .map_err(|_| {
                        CrossCompileError::CouldNotFindRequiredBinary(String::from(ld_binary_name))
                    })?
                    .into_os_string(),
            ),
            (
                OsString::from("CC_x86_64_unknown_linux_musl"),
                which(cc_binary_name)
                    .map_err(|_| {
                        CrossCompileError::CouldNotFindRequiredBinary(String::from(cc_binary_name))
                    })?
                    .into_os_string(),
            ),
        ])
    } else {
        Ok(vec![])
    }
}

#[derive(Debug)]
pub enum CrossCompileError {
    CouldNotFindRequiredBinary(String),
}

/// Returns a human-readable help text about cross compiling from the user's host platform to
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
