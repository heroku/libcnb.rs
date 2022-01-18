use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

pub(crate) struct PackBuildCommand {
    builder: String,
    path: PathBuf,
    image_name: String,
    buildpacks: Vec<BuildpackReference>,
    verbose: bool,
}

pub enum BuildpackReference {
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
            verbose: false,
        }
    }

    pub fn buildpack(&mut self, b: impl Into<BuildpackReference>) -> &mut Self {
        self.buildpacks.push(b.into());
        self
    }
}

impl From<PackBuildCommand> for Command {
    fn from(pack_build_command: PackBuildCommand) -> Self {
        let mut command = Command::new("pack");

        let mut args = vec![
            "build",
            &pack_build_command.image_name,
            "--builder",
            &pack_build_command.builder,
            "--path",
            pack_build_command.path.to_str().unwrap(),
        ];

        for buildpack in &pack_build_command.buildpacks {
            args.push("--buildpack");

            match buildpack {
                BuildpackReference::Id(id) => {
                    args.push(id);
                }
                BuildpackReference::Path(path_buf) => {
                    args.push(path_buf.to_str().unwrap());
                }
            }
        }

        if pack_build_command.verbose {
            args.push("-v");
        }

        command.args(args);

        command
    }
}
