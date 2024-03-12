//! Provides types and helpers to work with layers.

mod handling;
mod public_interface;

pub(crate) mod struct_api;
#[cfg(test)]
mod tests;

pub(crate) use handling::*;
pub use public_interface::*;
pub use struct_api::*;
