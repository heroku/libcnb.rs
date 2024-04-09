//! Provides types and helpers to work with layers.

pub(crate) mod shared;
pub(crate) mod struct_api;
pub(crate) mod trait_api;

pub use shared::DeleteLayerError;
pub use shared::LayerError;
pub use shared::ReadLayerError;
pub use shared::WriteLayerError;

pub use struct_api::*;
pub use trait_api::*;
