pub(crate) fn log<S: Into<String>>(message: S) {
    eprintln!("{}", message.into());
}

pub(crate) fn warn<S: Into<String>>(warning: S) {
    eprintln!("⚠️ {}", warning.into());
}

pub(crate) fn fail_with_error<S: Into<String>>(error: S) -> ! {
    eprintln!("❌ {}", error.into());
    std::process::exit(UNSPECIFIED_ERROR);
}

const UNSPECIFIED_ERROR: i32 = 1;
