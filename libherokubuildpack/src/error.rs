use crate::log::log_error;
use std::fmt::Debug;

/// Handles a given [`libcnb::Error`] in a consistent style.
///
/// This function is intended to be used inside [`libcnb::Buildpack::on_error`].
///
/// It outputs generic libcnb errors in a consistent style using the [logging functions](log_error) from this
/// crate. Buildpack specific errors are handled by the passed custom handler.
///
/// # Example:
/// ```
/// use libcnb::build::{BuildContext, BuildResult};
/// use libcnb::Buildpack;
/// use libcnb::detect::{DetectContext, DetectResult};
/// use libcnb::generic::{GenericMetadata, GenericPlatform};
/// use libherokubuildpack::log::log_error;
/// use libherokubuildpack::error::on_error;
///
/// #[derive(Debug)]
/// enum FooBuildpackError {
///     CannotExecuteFooBuildTool(std::io::Error),
///     InvalidFooDescriptorToml
/// }
///
/// fn on_foo_buildpack_error(e: FooBuildpackError) {
///     match e {
///         FooBuildpackError::InvalidFooDescriptorToml => {
///             log_error("Invalid foo.toml", "Your app's foo.toml is invalid!");
///         }
///         FooBuildpackError::CannotExecuteFooBuildTool(inner) => {
///             log_error("Couldn't execute foo build tool", format!("Cause: {}", &inner));
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
            log_error("Internal Buildpack Error", libcnb_error.to_string());
        }
    }
}
