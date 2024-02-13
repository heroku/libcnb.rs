use crate::buildpack_output::BuildpackOutput;
use std::fmt::Debug;

/// Handles a given [`libcnb::Error`] in a consistent style.
///
/// This function is intended to be used inside [`libcnb::Buildpack::on_error`].
///
/// It outputs generic libcnb errors in a consistent style using [`BuildpackOutput`] from this
/// crate. Buildpack specific errors are handled by the passed custom handler.
///
/// # Example:
/// ```
/// use libcnb::build::{BuildContext, BuildResult};
/// use libcnb::Buildpack;
/// use libcnb::detect::{DetectContext, DetectResult};
/// use libcnb::generic::{GenericMetadata, GenericPlatform};
/// use libherokubuildpack::buildpack_output::BuildpackOutput;
/// use libherokubuildpack::error::on_error;
///
/// #[derive(Debug)]
/// enum FooBuildpackError {
///     CannotExecuteFooBuildTool(std::io::Error),
///     InvalidFooDescriptorToml
/// }
///
/// fn on_foo_buildpack_error(e: FooBuildpackError) {
///     let output = BuildpackOutput::new(std::io::stdout()).start_silent();
///     match e {
///         FooBuildpackError::InvalidFooDescriptorToml => {
///             output.error("Invalid foo.toml\n\nYour app's foo.toml is invalid!");
///         }
///         FooBuildpackError::CannotExecuteFooBuildTool(inner) => {
///             output.error(format!("Couldn't execute foo build tool\n\nYour app's foo.toml is invalid!\n\nCause: {}", &inner));
///         }
///     }
/// }
///
/// struct FooBuildpack;
///
/// impl Buildpack for FooBuildpack {
///     type Platform = GenericPlatform;
///     type Metadata = GenericMetadata;
///     type Error = FooBuildpackError;
///
///     // Omitted detect and build implementations...
///     # fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
///     #     unimplemented!()
///     # }
///     #
///     # fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
///     #     unimplemented!()
///     # }
///
///     fn on_error(&self, error: libcnb::Error<Self::Error>) {
///         on_error(on_foo_buildpack_error, error)
///     }
/// }
/// ```
pub fn on_error<F, E>(f: F, error: libcnb::Error<E>)
where
    E: Debug,
    F: Fn(E),
{
    match error {
        libcnb::Error::BuildpackError(buildpack_error) => f(buildpack_error),
        libcnb_error => {
            BuildpackOutput::new(std::io::stdout())
                .start_silent()
                .error(format!("Internal Buildpack Error\n\n{libcnb_error}"));
        }
    }
}
