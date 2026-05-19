use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

/// Represents a volume mount for Docker containers.
#[derive(Clone, Debug)]
pub(crate) struct VolumeMount {
    pub(crate) source: PathBuf,
    pub(crate) target: PathBuf,
    pub(crate) options: Option<String>,
}

/// Represents a `pack build` command.
#[derive(Clone, Debug)]
pub(crate) struct PackBuildCommand {
    build_cache_volume_name: String,
    builder: String,
    buildpacks: Vec<BuildpackReference>,
    env: BTreeMap<String, String>,
    image_name: String,
    launch_cache_volume_name: String,
    path: PathBuf,
    pull_policy: PullPolicy,
    trust_builder: bool,
    trust_extra_buildpacks: bool,
    volumes: Vec<VolumeMount>,
}

#[derive(Clone, Debug)]
pub(crate) enum BuildpackReference {
    Id(String),
    Path(PathBuf),
}

impl From<PathBuf> for BuildpackReference {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

impl From<String> for BuildpackReference {
    fn from(id: String) -> Self {
        Self::Id(id)
    }
}

#[derive(Clone, Debug)]
/// Controls whether Pack should pull images.
#[allow(dead_code)]
pub(crate) enum PullPolicy {
    /// Always pull images.
    Always,
    /// Use local images if they are already present, rather than pulling updated images.
    IfNotPresent,
    /// Never pull images. If the required images are not already available locally the pack command will fail.
    Never,
}

impl PackBuildCommand {
    pub(crate) fn new(
        builder: impl Into<String>,
        path: impl Into<PathBuf>,
        image_name: impl Into<String>,
        build_cache_volume_name: impl Into<String>,
        launch_cache_volume_name: impl Into<String>,
    ) -> Self {
        Self {
            build_cache_volume_name: build_cache_volume_name.into(),
            builder: builder.into(),
            buildpacks: Vec::new(),
            env: BTreeMap::new(),
            image_name: image_name.into(),
            launch_cache_volume_name: launch_cache_volume_name.into(),
            path: path.into(),
            // Prevent redundant image-pulling, which slows tests and risks hitting registry rate limits.
            pull_policy: PullPolicy::IfNotPresent,
            trust_builder: true,
            trust_extra_buildpacks: true,
            volumes: Vec::new(),
        }
    }

    pub(crate) fn buildpack(&mut self, b: impl Into<BuildpackReference>) -> &mut Self {
        self.buildpacks.push(b.into());
        self
    }

    pub(crate) fn env(&mut self, k: impl Into<String>, v: impl Into<String>) -> &mut Self {
        self.env.insert(k.into(), v.into());
        self
    }

    pub(crate) fn volume(&mut self, v: VolumeMount) -> &mut Self {
        self.volumes.push(v);
        self
    }
}

impl From<PackBuildCommand> for Command {
    fn from(pack_build_command: PackBuildCommand) -> Self {
        let mut command = Self::new("pack");

        command.args([
            "build",
            &pack_build_command.image_name,
            "--builder",
            &pack_build_command.builder,
            "--cache",
            &format!(
                "type=build;format=volume;name={}",
                pack_build_command.build_cache_volume_name
            ),
            "--cache",
            &format!(
                "type=launch;format=volume;name={}",
                pack_build_command.launch_cache_volume_name
            ),
            "--path",
            &pack_build_command.path.to_string_lossy(),
            "--pull-policy",
            match pack_build_command.pull_policy {
                PullPolicy::Always => "always",
                PullPolicy::IfNotPresent => "if-not-present",
                PullPolicy::Never => "never",
            },
        ]);

        for buildpack in pack_build_command.buildpacks {
            command.args([
                "--buildpack",
                &match buildpack {
                    BuildpackReference::Id(id) => id,
                    BuildpackReference::Path(path_buf) => path_buf.to_string_lossy().to_string(),
                },
            ]);
        }

        for (env_key, env_value) in &pack_build_command.env {
            command.args(["--env", &format!("{env_key}={env_value}")]);
        }

        if pack_build_command.trust_builder {
            command.arg("--trust-builder");
        }

        if pack_build_command.trust_extra_buildpacks {
            command.arg("--trust-extra-buildpacks");
        }

        for volume in &pack_build_command.volumes {
            let mut arg = OsString::from(&volume.source);
            arg.push(":");
            arg.push(&volume.target);
            if let Some(ref opts) = volume.options {
                arg.push(":");
                arg.push(opts);
            }
            command.arg("--volume");
            command.arg(arg);
        }

        command
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PackSbomDownloadCommand {
    image_name: String,
    output_dir: Option<PathBuf>,
}

/// Represents a `pack sbom download` command.
impl PackSbomDownloadCommand {
    pub(crate) fn new(image_name: impl Into<String>) -> Self {
        Self {
            image_name: image_name.into(),
            output_dir: None,
        }
    }

    pub(crate) fn output_dir(&mut self, output_dir: impl Into<PathBuf>) -> &mut Self {
        self.output_dir = Some(output_dir.into());
        self
    }
}

impl From<PackSbomDownloadCommand> for Command {
    fn from(pack_command: PackSbomDownloadCommand) -> Self {
        let mut command = Self::new("pack");

        command.args(["sbom", "download", &pack_command.image_name]);

        if let Some(output_dir) = pack_command.output_dir {
            command.args(["--output-dir", &output_dir.to_string_lossy()]);
        }

        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn from_pack_build_command_to_command() {
        let mut input = PackBuildCommand {
            build_cache_volume_name: String::from("build-cache-volume"),
            builder: String::from("builder:20"),
            buildpacks: vec![
                BuildpackReference::Id(String::from("libcnb/buildpack1")),
                BuildpackReference::Path(PathBuf::from("/tmp/buildpack2")),
            ],
            env: BTreeMap::from([
                (String::from("ENV_FOO"), String::from("FOO_VALUE")),
                (String::from("ENV_BAR"), String::from("WHITESPACE VALUE")),
            ]),
            image_name: String::from("my-image"),
            launch_cache_volume_name: String::from("launch-cache-volume"),
            path: PathBuf::from("/tmp/foo/bar"),
            pull_policy: PullPolicy::IfNotPresent,
            trust_builder: true,
            trust_extra_buildpacks: true,
            volumes: Vec::new(),
        };

        let command: Command = input.clone().into();

        assert_eq!(command.get_program(), "pack");

        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            [
                "build",
                "my-image",
                "--builder",
                "builder:20",
                "--cache",
                "type=build;format=volume;name=build-cache-volume",
                "--cache",
                "type=launch;format=volume;name=launch-cache-volume",
                "--path",
                "/tmp/foo/bar",
                "--pull-policy",
                "if-not-present",
                "--buildpack",
                "libcnb/buildpack1",
                "--buildpack",
                "/tmp/buildpack2",
                "--env",
                "ENV_BAR=WHITESPACE VALUE",
                "--env",
                "ENV_FOO=FOO_VALUE",
                "--trust-builder",
                "--trust-extra-buildpacks",
            ]
        );

        assert_eq!(command.get_envs().collect::<Vec<_>>(), Vec::new());

        // Assert conditional '--trust-builder' flag works as expected:
        input.trust_builder = false;
        let command: Command = input.clone().into();
        assert!(
            !command
                .get_args()
                .any(|arg| arg == OsStr::new("--trust-builder"))
        );
    }

    #[test]
    fn from_pack_sbom_download_command_to_command() {
        let mut input = PackSbomDownloadCommand {
            image_name: String::from("my-image"),
            output_dir: None,
        };

        let command: Command = input.clone().into();

        assert_eq!(command.get_program(), "pack");

        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["sbom", "download", "my-image"]
        );

        assert_eq!(command.get_envs().collect::<Vec<_>>(), Vec::new());

        // Assert conditional '--output-dir' flag works as expected:
        input.output_dir = Some(PathBuf::from("/tmp/sboms"));
        let command: Command = input.into();

        assert_eq!(command.get_program(), "pack");

        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["sbom", "download", "my-image", "--output-dir", "/tmp/sboms"]
        );

        assert_eq!(command.get_envs().collect::<Vec<_>>(), Vec::new());
    }

    #[test]
    fn from_pack_build_command_with_volumes() {
        let mut input = PackBuildCommand::new(
            "builder:22",
            "/tmp/app",
            "my-image",
            "build-cache",
            "launch-cache",
        );
        input.volume(VolumeMount {
            source: PathBuf::from("/host/path"),
            target: PathBuf::from("/container/path"),
            options: None,
        });
        input.volume(VolumeMount {
            source: PathBuf::from("/other"),
            target: PathBuf::from("/mnt"),
            options: Some(String::from("ro")),
        });

        let command: Command = input.into();

        let args: Vec<&OsStr> = command.get_args().collect();
        assert!(args.contains(&OsStr::new("--volume")));
        assert!(args.contains(&OsStr::new("/host/path:/container/path")));
        assert!(args.contains(&OsStr::new("/other:/mnt:ro")));
    }
}
