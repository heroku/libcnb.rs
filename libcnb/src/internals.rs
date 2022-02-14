// Used by the libcnb::additional_buildpack_binary_path macro.
//
// We cannot use `::libcnb_proc_macros::verify_bin_target_exists` in our macros directly as this
// would require every crate to explicitly import the `libcnb_proc_macros` crate as crates can't
// use code from transitive dependencies.
pub use libcnb_proc_macros::verify_bin_target_exists;
