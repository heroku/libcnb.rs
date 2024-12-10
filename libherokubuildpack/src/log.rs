use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// # Panics
///
/// Will panic if there was a problem setting the color settings, or all bytes could
/// not be written due to either I/O errors or EOF being reached.
// TODO: Replace `.unwrap()` usages with `.expect()` to give a clearer error message:
// https://github.com/heroku/libcnb.rs/issues/712
#[allow(clippy::unwrap_used)]
pub fn log_error(header: impl AsRef<str>, body: impl AsRef<str>) {
    let mut stream = StandardStream::stderr(ColorChoice::Always);
    write_styled_message(
        &mut stream,
        format!("\n[Error: {}]", header.as_ref()),
        ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true),
    )
    .unwrap();

    write_styled_message(&mut stream, body, ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
    stream.flush().unwrap();
}

/// # Panics
///
/// Will panic if there was a problem setting the color settings, or all bytes could
/// not be written due to either I/O errors or EOF being reached.
// TODO: Replace `.unwrap()` usages with `.expect()` to give a clearer error message:
// https://github.com/heroku/libcnb.rs/issues/712
#[allow(clippy::unwrap_used)]
pub fn log_warning(header: impl AsRef<str>, body: impl AsRef<str>) {
    let mut stream = StandardStream::stderr(ColorChoice::Always);
    write_styled_message(
        &mut stream,
        format!("\n[Warning: {}]", header.as_ref()),
        ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true),
    )
    .unwrap();

    write_styled_message(
        &mut stream,
        body,
        ColorSpec::new().set_fg(Some(Color::Yellow)),
    )
    .unwrap();
    stream.flush().unwrap();
}

/// # Panics
///
/// Will panic if there was a problem setting the color settings, or all bytes could
/// not be written due to either I/O errors or EOF being reached.
// TODO: Replace `.unwrap()` usages with `.expect()` to give a clearer error message:
// https://github.com/heroku/libcnb.rs/issues/712
#[allow(clippy::unwrap_used)]
pub fn log_header(title: impl AsRef<str>) {
    let mut stream = StandardStream::stdout(ColorChoice::Always);
    write_styled_message(
        &mut stream,
        format!("\n[{}]", title.as_ref()),
        ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true),
    )
    .unwrap();
    stream.flush().unwrap();
}

/// # Panics
///
/// Will panic if all bytes could not be written due to I/O errors or EOF being reached.
// TODO: Replace `.unwrap()` usages with `.expect()` to give a clearer error message:
// https://github.com/heroku/libcnb.rs/issues/712
#[allow(clippy::unwrap_used)]
pub fn log_info(message: impl AsRef<str>) {
    println!("{}", message.as_ref());
    std::io::stdout().flush().unwrap();
}

// Styles each line of text separately, so that when buildpack output is streamed to the
// user (and prefixes like `remote:` added) the line colour doesn't leak into the prefixes.
fn write_styled_message(
    stream: &mut StandardStream,
    message: impl AsRef<str>,
    spec: &ColorSpec,
) -> io::Result<()> {
    // Using `.split('\n')` rather than `.lines()` since the latter eats trailing newlines in
    // the passed message, which would (a) prevent the caller from being able to add spacing at
    // the end of their message and (b) differ from the `println!` / `writeln!` behaviour.
    for line in message.as_ref().split('\n') {
        stream.set_color(spec)?;
        write!(stream, "{line}")?;
        stream.reset()?;
        writeln!(stream)?;
    }
    Ok(())
}
