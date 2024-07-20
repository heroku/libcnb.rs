use std::io::Write;
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
    stream
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
        .unwrap();
    writeln!(&mut stream, "\n[Error: {}]", header.as_ref()).unwrap();

    stream
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
        .unwrap();
    writeln!(&mut stream, "{}", body.as_ref()).unwrap();
    stream.reset().unwrap();
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
    stream
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))
        .unwrap();
    writeln!(&mut stream, "\n[Warning: {}]", header.as_ref()).unwrap();

    stream
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))
        .unwrap();
    writeln!(&mut stream, "{}", body.as_ref()).unwrap();
    stream.reset().unwrap();
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
    stream
        .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))
        .unwrap();
    writeln!(&mut stream, "\n[{}]", title.as_ref()).unwrap();
    stream.reset().unwrap();
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
