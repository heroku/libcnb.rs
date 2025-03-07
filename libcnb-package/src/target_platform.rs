use std::fmt::Display;

#[derive(Clone)]
pub struct TargetPlatform {
    pub os: TargetOs,
    pub arch: TargetArch,
}

impl TargetPlatform {
    #[must_use]
    pub fn new(os: TargetOs, arch: TargetArch) -> Self {
        TargetPlatform { os, arch }
    }
}

impl Default for TargetPlatform {
    fn default() -> Self {
        TargetPlatform::new(TargetOs::Linux, default_arch())
    }
}

impl From<&TargetPlatform> for String {
    fn from(value: &TargetPlatform) -> Self {
        value.to_string()
    }
}

impl Display for TargetPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let os = match self.os {
            TargetOs::Linux => "linux",
        };
        let arch = match self.arch {
            TargetArch::Amd64 => "amd64",
            TargetArch::Arm64 => "arm64",
        };
        write!(f, "{os}/{arch}")
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TargetOs {
    Linux,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TargetArch {
    Amd64,
    Arm64,
}

fn default_arch() -> TargetArch {
    match std::env::consts::ARCH {
        "amd64" | "x86_64" => TargetArch::Amd64,
        "arm64" | "aarch64" => TargetArch::Arm64,
        _ => panic!("Unsupported target arch: {}", std::env::consts::ARCH),
    }
}
