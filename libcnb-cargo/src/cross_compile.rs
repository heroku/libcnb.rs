use std::ffi::OsString;
use which::which;

const MUSL_TARGET: &str = "x86_64-unknown-linux-musl";
const MAC_BINARIES: CompilerBinaries = CompilerBinaries {
    ld: Binary("x86_64-linux-musl-ld"),
    cc: Binary("x86_64-linux-musl-gcc"),
};
const LINUX_BINARIES: CompilerBinaries = CompilerBinaries {
    ld: Binary("musl-ld"),
    cc: Binary("musl-gcc"),
};

#[derive(Debug)]
pub enum CrossCompileError {
    CouldNotFindRequiredBinary(String),
}

/// Constructs a set of environment variables to enable cross-compiling from the user's host
/// platform to the desired target platform.
///
/// This function will not install required toolchains, linkers or compilers automatically. It will
/// look for the required tools and returns an error if they cannot be found. See
/// [`cross_compile_help`] on how to support users setting up their machine for cross-compilation.
///
/// # Errors
/// Will return `Err` if the cross-compile environment could not be created for the current host
/// and target platform.
pub fn cross_compile_env(
    target_triple: impl AsRef<str>,
) -> Result<Vec<(OsString, OsString)>, CrossCompileError> {
    if target_triple.as_ref() == MUSL_TARGET {
        if cfg!(target_os = "macos") {
            return Ok(vec![
                (
                    OsString::from("CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"),
                    MAC_BINARIES.ld.which()?,
                ),
                (
                    OsString::from("CC_x86_64_unknown_linux_musl"),
                    MAC_BINARIES.cc.which()?,
                ),
            ]);
        } else if cfg!(target_os = "linux") {
            // Do we want to set the env vars here?
            LINUX_BINARIES.ld.which()?;
            LINUX_BINARIES.cc.which()?;
        }
    }

    Ok(vec![])
}

/// Returns a human-readable help text about cross-compiling from the user's host platform to
/// the desired target platform.
pub fn cross_compile_help(target_triple: impl AsRef<str>) -> Option<String> {
    if target_triple.as_ref() == MUSL_TARGET {
        if cfg!(target_os = "macos") {
            return Some(String::from(
                r#"For cross-compilation from macOS to x86_64-unknown-linux-musl, a C compiler and linker for the
target platform must be installed on your computer.

The easiest way to install 'x86_64-linux-musl-ld' and 'x86_64-linux-musl-gcc', is to follow the
instructions in the linked GitHub repository:

https://github.com/FiloSottile/homebrew-musl-cross"#,
            ));
        } else if cfg!(target_os = "linux") {
            return Some(String::from(
                r#"For cross-compilation from macOS to x86_64-unknown-linux-musl, a C compiler and linker for the
target platform must be installed on your computer.

The easiest way to install 'musl-ld' and 'musl-gcc' on Debian/Ubuntu is to install 'musl-tools' or equivalent for your distro."#,
            ));
        }
    }

    None
}

/// Binaries used for linking and compiling for cross compilation
struct CompilerBinaries {
    pub ld: Binary,
    pub cc: Binary,
}

/// Newtype for finding a binary on the PATH
struct Binary(&'static str);

impl Binary {
    pub fn which(&self) -> Result<OsString, CrossCompileError> {
        Ok(which(self.0)
            .map_err(|_| CrossCompileError::CouldNotFindRequiredBinary(String::from(self.0)))?
            .into_os_string())
    }
}

impl std::ops::Deref for Binary {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
