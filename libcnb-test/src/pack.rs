use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Represents a `pack build` command.
#[derive(Clone, Debug)]
pub(crate) struct PackBuildCommand {
    builder: String,
    path: PathBuf,
    image_name: String,
    buildpacks: Vec<BuildpackReference>,
    env: BTreeMap<String, String>,
    verbose: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum BuildpackReference {
    Id(String),
    Path(PathBuf),
}

impl From<PathBuf> for BuildpackReference {
    fn from(path: PathBuf) -> Self {
        BuildpackReference::Path(path)
    }
}

impl From<&TempDir> for BuildpackReference {
    fn from(path: &TempDir) -> Self {
        BuildpackReference::Path(path.path().into())
    }
}

impl From<String> for BuildpackReference {
    fn from(id: String) -> Self {
        BuildpackReference::Id(id)
    }
}

impl PackBuildCommand {
    pub fn new(
        builder: impl Into<String>,
        path: impl Into<PathBuf>,
        image_name: impl Into<String>,
    ) -> PackBuildCommand {
        PackBuildCommand {
            builder: builder.into(),
            path: path.into(),
            image_name: image_name.into(),
            buildpacks: vec![],
            env: BTreeMap::new(),
            verbose: false,
        }
    }

    pub fn buildpack(&mut self, b: impl Into<BuildpackReference>) -> &mut Self {
        self.buildpacks.push(b.into());
        self
    }

    pub fn env(&mut self, k: impl Into<String>, v: impl Into<String>) -> &mut Self {
        self.env.insert(k.into(), v.into());
        self
    }
}

impl From<PackBuildCommand> for Command {
    fn from(pack_build_command: PackBuildCommand) -> Self {
        let mut command = Command::new("pack");

        let mut args = vec![
            String::from("build"),
            pack_build_command.image_name,
            String::from("--builder"),
            pack_build_command.builder,
            String::from("--path"),
            pack_build_command.path.to_string_lossy().to_string(),
        ];

        for buildpack in pack_build_command.buildpacks {
            args.push(String::from("--buildpack"));

            match buildpack {
                BuildpackReference::Id(id) => {
                    args.push(id);
                }
                BuildpackReference::Path(path_buf) => {
                    args.push(path_buf.to_string_lossy().to_string());
                }
            }
        }

        for (env_key, env_value) in &pack_build_command.env {
            args.push(String::from("--env"));
            args.push(format!("{env_key}={env_value}"));
        }

        if pack_build_command.verbose {
            args.push(String::from("-v"));
        }

        command.args(args);

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
            builder: String::from("builder:20"),
            path: PathBuf::from("/tmp/foo/bar"),
            image_name: String::from("my-image"),
            buildpacks: vec![
                BuildpackReference::Id(String::from("libcnb/buildpack1")),
                BuildpackReference::Path(PathBuf::from("/tmp/buildpack2")),
            ],
            env: BTreeMap::from([
                (String::from("ENV_FOO"), String::from("FOO_VALUE")),
                (String::from("ENV_BAR"), String::from("WHITESPACE VALUE")),
            ]),
            verbose: true,
        };

        let command: Command = input.clone().into();

        assert_eq!(command.get_program(), "pack");

        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            vec![
                "build",
                "my-image",
                "--builder",
                "builder:20",
                "--path",
                "/tmp/foo/bar",
                "--buildpack",
                "libcnb/buildpack1",
                "--buildpack",
                "/tmp/buildpack2",
                "--env",
                "ENV_BAR=WHITESPACE VALUE",
                "--env",
                "ENV_FOO=FOO_VALUE",
                "-v"
            ]
        );

        assert_eq!(command.get_envs().collect::<Vec<_>>(), vec![]);

        // Assert conditional '-v' flag works as expected:
        input.verbose = false;
        let command: Command = input.into();
        assert!(!command.get_args().any(|arg| arg == OsStr::new("-v")));
    }
}
