//! Helpers for formatting and colorizing your output.

use crate::buildpack_output::ansi_escape::{self, ANSI};

/// Decorate a URL for the build output.
pub fn url(contents: impl AsRef<str>) -> String {
    ansi_escape::inject_default_ansi_escape(&ANSI::BoldCyan, contents)
}

/// Decorate the name of a command being run i.e. `bundle install`.
pub fn command(contents: impl AsRef<str>) -> String {
    value(ansi_escape::inject_default_ansi_escape(
        &ANSI::BoldCyan,
        contents,
    ))
}

/// Decorate an important value i.e. `2.3.4`.
pub fn value(contents: impl AsRef<str>) -> String {
    let contents = ansi_escape::inject_default_ansi_escape(&ANSI::Yellow, contents);
    format!("`{contents}`")
}

/// Decorate additional information at the end of a line.
pub fn details(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    format!("({contents})")
}
