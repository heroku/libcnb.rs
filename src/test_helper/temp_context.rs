use crate::{
    data::buildpack::BuildpackToml, data::buildpack_plan::BuildpackPlan,
    data::buildpack_plan::Entry, BuildContext, DetectContext, GenericBuildContext,
    GenericDetectContext, GenericMetadata, GenericPlatform, Platform,
};

/// Creates build and detect contexts with references to temp directories for testing
///
/// When the context goes out of scope, the directories are cleaned from disk.
///
///```
/// use libcnb::test_helper::TempContext;
/// use std::fs;
///
/// pub fn build(context: libcnb::GenericBuildContext) {}
/// pub fn detect(context: libcnb::GenericDetectContext) {}
///
/// let temp = TempContext::new("heroku-20", include_str!("../../examples/example-02-ruby-sample/buildpack.toml"));
/// let build_context = temp.build;
/// let detect_context = temp.detect;
/// let launch_toml_path = build_context.layers_dir.join("launch.toml");
///
/// std::fs::write(build_context.app_dir.join("Procfile"), "web: bundle exec rails s").unwrap();
///
/// // Pass to build to assert behavior
/// build(build_context);
///
/// // Pass to detect to assert behavior
/// detect(detect_context);
///```
pub struct TempContext {
    pub detect: GenericDetectContext,
    pub build: GenericBuildContext,
    _tmp_dir: tempfile::TempDir,
}

impl TempContext {
    pub fn new(stack_id: impl AsRef<str>, buildpack_toml_string: impl AsRef<str>) -> Self {
        let buildpack_toml_string = buildpack_toml_string.as_ref();
        let tmp_dir = tempfile::tempdir().unwrap();
        let app_dir = tmp_dir.path().join("app");
        let layers_dir = tmp_dir.path().join("layers");
        let platform_dir = tmp_dir.path().join("platform");
        let buildpack_dir = tmp_dir.path().join("buildpack");

        for dir in [&app_dir, &layers_dir, &buildpack_dir, &platform_dir] {
            std::fs::create_dir_all(dir).unwrap();
        }

        let stack_id = String::from(stack_id.as_ref());
        let platform = GenericPlatform::from_path(&platform_dir).unwrap();
        let buildpack_descriptor: BuildpackToml<GenericMetadata> =
            toml::from_str(buildpack_toml_string).unwrap();

        let detect_context = DetectContext {
            platform,
            buildpack_descriptor,
            app_dir: app_dir.clone(),
            buildpack_dir: buildpack_dir.clone(),
            stack_id: stack_id.clone(),
        };

        let platform = GenericPlatform::from_path(&platform_dir).unwrap();
        let buildpack_descriptor: BuildpackToml<GenericMetadata> =
            toml::from_str(buildpack_toml_string).unwrap();
        let buildpack_plan = BuildpackPlan {
            entries: Vec::<Entry>::new(),
        };
        let build_context = BuildContext {
            layers_dir,
            app_dir,
            buildpack_dir,
            stack_id,
            platform,
            buildpack_plan,
            buildpack_descriptor,
        };

        TempContext {
            detect: detect_context,
            build: build_context,
            _tmp_dir: tmp_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directories_are_created() {
        let temp = TempContext::new("heroku-20", fake_buildpack_toml());
        assert!(temp.build.app_dir.exists());
        assert!(temp.build.layers_dir.exists());
        assert!(temp.build.buildpack_dir.exists());

        assert!(temp.detect.app_dir.exists());
        assert!(temp.detect.buildpack_dir.exists());
    }

    fn fake_buildpack_toml() -> &'static str {
        r#"
            api = "0.4"

            [buildpack]
            id = "com.examples.buildpacks.ruby"
            version = "0.0.1"
            name = "Ruby Buildpack"

            [[stacks]]
            id = "io.buildpacks.stacks.bionic"
        "#
    }
}
