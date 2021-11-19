// This macro is used by all newtype literal macros to verify if the value matches the regex. It
// is not intended to be used outside of this crate. But since the code that macros expand to is
// just regular code, we need to expose this to users of this crate.
//
// We cannot use `::libcnb_proc_macros::verify_regex` in our macros directly as this would require
// every crate to explicitly import the `libcnb_proc_macros` crate as crates can't use code from
// transitive dependencies.
pub use libcnb_proc_macros::verify_regex;
