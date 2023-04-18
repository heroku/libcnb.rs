pub(crate) fn log<IntoString: Into<String>>(message: IntoString) {
    eprintln!("{}", message.into());
}

pub(crate) fn warn<IntoString: Into<String>>(warning: IntoString) {
    eprintln!("⚠️ {}", warning.into());
}

pub(crate) fn fail_with_error<IntoString: Into<String>>(error: IntoString) -> ! {
    eprintln!("❌ {}", error.into());
    std::process::exit(UNSPECIFIED_ERROR);
}

const UNSPECIFIED_ERROR: i32 = 1;
