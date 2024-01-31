use crate::buildpack_output::ansi_escape::*;

/// Helpers for formatting and colorizing your output.

/// Decorate a URL for the build output.
#[must_use]
pub fn url(contents: impl AsRef<str>) -> String {
    colorize_multiline(CYAN, contents)
}

/// Decorate the name of a command being run i.e. `bundle install`.
#[must_use]
pub fn command(contents: impl AsRef<str>) -> String {
    value(colorize_multiline(BOLD_CYAN, contents.as_ref()))
}

/// Decorate an important value i.e. `2.3.4`.
#[must_use]
pub fn value(contents: impl AsRef<str>) -> String {
    let contents = colorize_multiline(YELLOW, contents.as_ref());
    format!("`{contents}`")
}

/// Decorate additional information at the end of a line.
#[must_use]
pub fn details(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    format!("({contents})")
}
