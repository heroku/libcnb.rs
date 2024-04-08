//! Provides types and helpers to work with layers.

pub(crate) mod shared;
pub(crate) mod struct_api;
pub(crate) mod trait_api;

pub use shared::LayerError;
pub use struct_api::*;
pub use trait_api::*;
