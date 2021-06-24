use std::error::Error;
use std::{env, fs, path::PathBuf, process};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::data::layer::LayerContentMetadata;
use crate::error::LibCnbError;
use crate::shared::{write_toml_file, TomlFileError};
use crate::{
    data::{buildpack::BuildpackToml, buildpack_plan::BuildpackPlan, launch::Launch},
    platform::Platform,
    shared::read_toml_file,
};

pub fn cnb_runtime_build<
    E: Error,
    F: Fn(BuildContext<P, BM>) -> Result<(), LibCnbError<E>>,
    BM: DeserializeOwned,
    P: Platform,
>(
    build_fn: F,
) -> Result<(), LibCnbError<E>> {
    let args = parse_build_args_or_exit();

    let layers_dir = args.layers_dir_path;

    let app_dir = env::current_dir().map_err(LibCnbError::CannotDetermineAppDirectory)?;

    let buildpack_dir = env::var("CNB_BUILDPACK_DIR")
        .map_err(LibCnbError::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(LibCnbError::CannotDetermineStackId)?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(LibCnbError::CannotCreatePlatformFromPath)?;

    let buildpack_plan =
        read_toml_file(&args.buildpack_plan_path).map_err(LibCnbError::CannotReadBuildpackPlan)?;

    let buildpack_descriptor = read_toml_file(buildpack_dir.join("buildpack.toml"))
        .map_err(LibCnbError::CannotReadBuildpackDescriptor)?;

    let context = BuildContext {
        layers_dir,
        app_dir,
        buildpack_dir,
        stack_id,
        platform,
        buildpack_plan,
        buildpack_descriptor,
    };

    build_fn(context)
}

struct BuildArgs {
    pub layers_dir_path: PathBuf,
    pub platform_dir_path: PathBuf,
    pub buildpack_plan_path: PathBuf,
}

fn parse_build_args_or_exit() -> BuildArgs {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, layers_dir_path, platform_dir_path, buildpack_plan_path] => BuildArgs {
            layers_dir_path: PathBuf::from(layers_dir_path),
            platform_dir_path: PathBuf::from(platform_dir_path),
            buildpack_plan_path: PathBuf::from(buildpack_plan_path),
        },
        _ => {
            eprintln!("Usage: build <layers> <platform> <plan>");
            eprintln!("https://github.com/buildpacks/spec/blob/main/buildpack.md#build");
            process::exit(1);
        }
    }
}

pub struct BuildContext<P: Platform, BM> {
    pub layers_dir: PathBuf,
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: P,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: BuildpackToml<BM>,
}

impl<P: Platform, BM> BuildContext<P, BM> {
    pub fn layer_path(&self, layer_name: impl AsRef<str>) -> PathBuf {
        self.layers_dir.join(layer_name.as_ref())
    }

    pub fn layer_content_metadata_path(&self, layer_name: impl AsRef<str>) -> PathBuf {
        self.layers_dir
            .join(format!("{}.toml", layer_name.as_ref()))
    }

    pub fn read_layer_content_metadata<M: DeserializeOwned>(
        &self,
        layer_name: impl AsRef<str>,
    ) -> Result<Option<LayerContentMetadata<M>>, TomlFileError> {
        let path = self.layer_content_metadata_path(layer_name);

        if path.exists() {
            read_toml_file(path).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn write_layer_content_metadata<M: Serialize>(
        &self,
        layer_name: impl AsRef<str>,
        layer_content_metadata: &LayerContentMetadata<M>,
    ) -> Result<(), TomlFileError> {
        write_toml_file(
            layer_content_metadata,
            self.layer_content_metadata_path(layer_name),
        )
    }

    pub fn delete_layer(&self, layer_name: impl AsRef<str>) -> Result<(), std::io::Error> {
        // Do not fail if the metadata file does not exist
        match fs::remove_file(self.layer_content_metadata_path(&layer_name)) {
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(io_error),
            },
            Ok(_) => Ok(()),
        }?;

        match fs::remove_dir_all(self.layer_path(&layer_name)) {
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(io_error),
            },
            Ok(_) => Ok(()),
        }?;

        Ok(())
    }

    pub fn read_layer<M: DeserializeOwned>(
        &self,
        layer_name: impl AsRef<str>,
    ) -> Result<Option<(PathBuf, LayerContentMetadata<M>)>, TomlFileError> {
        let layer_path = self.layer_path(&layer_name);

        self.read_layer_content_metadata(&layer_name)
            .map(|maybe_content_layer_metadata| {
                maybe_content_layer_metadata.and_then(
                    |layer_content_metadata: LayerContentMetadata<M>| {
                        if layer_path.exists() {
                            Some((layer_path, layer_content_metadata))
                        } else {
                            None
                        }
                    },
                )
            })
    }

    pub fn layer_exists(&self, layer_name: impl AsRef<str>) -> bool {
        let layer_path = self.layer_path(&layer_name);
        let content_metadata_path = self.layer_content_metadata_path(&layer_name);
        layer_path.exists() && content_metadata_path.exists()
    }

    pub fn write_launch(&self, data: Launch) -> Result<(), TomlFileError> {
        write_toml_file(&data, self.layers_dir.join("launch.toml"))
    }
}
