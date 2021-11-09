use crate::build::{BuildContext, BuildOutcome};
use crate::detect::{DetectContext, DetectOutcome};
use crate::Platform;
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display};

pub trait Buildpack {
    type Platform: Platform;
    type Metadata: DeserializeOwned;
    type Error: Debug + Display;

    fn detect(&self, context: DetectContext<Self>) -> crate::Result<DetectOutcome, Self::Error>;

    fn build(&self, context: BuildContext<Self>) -> crate::Result<BuildOutcome, Self::Error>;

    fn handle_error(&self, error: crate::Error<Self::Error>) -> i32 {
        eprintln!("Unhandled error:");
        eprintln!("> {}", error);
        eprintln!("Buildpack will exit!");
        100
    }
}
