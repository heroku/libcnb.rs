use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::layer::{Layer, LayerData};
use libcnb::{buildpack_main, Buildpack};
use libherokubuildpack::buildpack_output::{state, BuildpackOutput, LayerOutput};
use std::fmt::Debug;
use std::io::Write;

pub(crate) struct BasicBuildpack;

impl Buildpack for BasicBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let output = BuildpackOutput::new(std::io::stdout())
            .start("Basic buildpack")
            .section("Running layer");
        let out = context.handle_layer(
            layer_name!("simple"),
            SimpleLayer {
                output: Some(output),
            },
        );
        println!("Build runs on stack {}!", context.stack_id);
        BuildResultBuilder::new().build()
    }
}

struct SimpleLayer<W>
where
    W: Write + Debug + Send + Sync + 'static,
{
    output: Option<BuildpackOutput<state::Section, W>>,
}

impl<W> LayerOutput<W> for SimpleLayer<W>
where
    W: Write + Debug + Send + Sync + 'static,
{
    fn get(&mut self) -> BuildpackOutput<state::Section, W> {
        self.output
            .take()
            .expect("Output must be set to use this layer")
    }

    fn set(&mut self, output: BuildpackOutput<state::Section, W>) {
        self.output = Some(output);
    }
}

impl<W> Layer for SimpleLayer<W>
where
    W: Write + Debug + Send + Sync + 'static,
{
    type Buildpack = BasicBuildpack;
    type Metadata = GenericMetadata;

    fn existing_layer_strategy(
        &mut self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_data: &libcnb::layer::LayerData<Self::Metadata>,
    ) -> Result<libcnb::layer::ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        self.step("Examining layer strategy");
        Ok(libcnb::layer::ExistingLayerStrategy::Recreate)
    }

    fn update(
        &mut self,
        _context: &BuildContext<Self::Buildpack>,
        layer_data: &libcnb::layer::LayerData<Self::Metadata>,
    ) -> Result<libcnb::layer::LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error>
    {
        self.step("Calling update");
        libcnb::layer::LayerResultBuilder::new(layer_data.content_metadata.metadata.clone())
            .env(layer_data.env.clone())
            .build()
    }

    fn migrate_incompatible_metadata(
        &mut self,
        _context: &BuildContext<Self::Buildpack>,
        _metadata: &GenericMetadata,
    ) -> Result<
        libcnb::layer::MetadataMigration<Self::Metadata>,
        <Self::Buildpack as Buildpack>::Error,
    > {
        Ok(libcnb::layer::MetadataMigration::RecreateLayer)
    }

    fn types(&self) -> libcnb::data::layer_content_metadata::LayerTypes {
        LayerTypes {
            launch: true,
            build: true,
            cache: true,
        }
    }

    fn create(
        &mut self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_path: &std::path::Path,
    ) -> Result<libcnb::layer::LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error>
    {
        self.step("Calling create");

        libcnb::layer::LayerResultBuilder::new(GenericMetadata::default()).build()
    }
}

buildpack_main!(BasicBuildpack);
