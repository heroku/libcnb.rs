use std::env::consts;
use std::ffi::OsString;
use which::which;

/// Provides assistance for cross-compiling from the user's host platform to the desired target platform.
///
/// This function will not install required toolchains, linkers or compilers automatically. It will
/// look for the required tools and returns a human-readable help text if they can't be found or
/// any other issue has been detected.
pub fn cross_compile_assistance(target_triple: impl AsRef<str>) -> CrossCompileAssistance {
    let target = target_triple.as_ref();
    let (gcc_path, help_text) = match (target, consts::OS, consts::ARCH) {
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_X86_64) => (
            "aarch64-linux-gnu-gcc",
            "Install aarch64 cross-compiler:\nsudo apt-get install g++-aarch64-linux-gnu",
        ),
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_MACOS, _) => (
            "aarch64-unknown-linux-musl-gcc",
            "Install aarch64 cross-compiler on macOS:\nbrew install messense/macos-cross-toolchains/aarch64-unknown-linux-musl",
        ),
        (AARCH64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_AARCH64) | (X86_64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_X86_64) => (
            "musl-gcc",
            "Install musl-tools:\nsudo apt-get install musl-tools",
        ),
        (X86_64_UNKNOWN_LINUX_MUSL, OS_LINUX, ARCH_AARCH64) => (
            "x86_64-linux-gnu-gcc",
            "Install x86_64 cross-compiler:\nsudo apt-get install g++-x86_64-linux-gnu",
        ),
        (X86_64_UNKNOWN_LINUX_MUSL, OS_MACOS, _) => (
            "x86_64-unknown-linux-musl-gcc",
            "Install x86_64 cross-compiler on macOS:\nbrew install messense/macos-cross-toolchains/x86_64-unknown-linux-musl",
        ),
        _ => return CrossCompileAssistance::NoAssistance,
        };
    generate_assistance(gcc_path, help_text, target)
}

fn generate_assistance(
    gcc_path: &str,
    help_text: &str,
    target_triple: &str,
) -> CrossCompileAssistance {
    let cargo_target_linker_var = format!(
        "CARGO_TARGET_{}_LINKER",
        target_triple.to_uppercase().replace('-', "_")
    );

    if which(gcc_path).is_err() {
        CrossCompileAssistance::HelpText(help_text.to_string())
    } else if gcc_path == "musl-gcc" {
        CrossCompileAssistance::Configuration {
            cargo_env: Vec::new(),
        }
    } else {
        let cargo_env = vec![
            (
                // Required until Cargo can auto-detect the musl-cross gcc/linker itself,
                // since otherwise it checks for a binary named 'musl-gcc' (which is handled above):
                // https://github.com/rust-lang/cargo/issues/4133
                OsString::from(cargo_target_linker_var),
                OsString::from(gcc_path),
            ),
            (
                // Required so that any crates that call out to gcc are also cross-compiled:
                // https://github.com/alexcrichton/cc-rs/issues/82
                OsString::from(format!("CC_{target_triple}")),
                OsString::from(gcc_path),
            ),
        ];
        CrossCompileAssistance::Configuration { cargo_env }
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

// Constants for OS and ARCH
const OS_LINUX: &str = "linux";
const OS_MACOS: &str = "macos";
const ARCH_X86_64: &str = "x86_64";
const ARCH_AARCH64: &str = "aarch64";
