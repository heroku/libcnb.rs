/// Removes [`std::io::Error`] values from a [`Result`] that have the
/// [`std::io::ErrorKind::NotFound`] error kind by replacing them with the default value for `T`.
pub(crate) fn default_on_not_found<T: Default>(
    result: Result<T, std::io::Error>,
) -> Result<T, std::io::Error> {
    match result {
        Err(io_error) => match io_error.kind() {
            std::io::ErrorKind::NotFound => Ok(T::default()),
            _ => Err(io_error),
        },
        other => other,
    }
}

#[cfg(test)]
mod test {
    use crate::util::default_on_not_found;
    use std::io::ErrorKind;

    #[test]
    fn default_on_not_found_with_notfound() {
        let not_found_io_error = std::io::Error::from(ErrorKind::NotFound);

        assert_eq!(
            default_on_not_found::<Option<String>>(Err(not_found_io_error)).unwrap(),
            None
        );
    }

    #[test]
    fn default_on_not_found_with_brokenpipe() {
        let broken_pipe_io_error = std::io::Error::from(ErrorKind::BrokenPipe);

        assert!(default_on_not_found::<Option<String>>(Err(broken_pipe_io_error)).is_err());
    }

    #[test]
    fn default_on_not_found_with_ok() {
        assert_eq!(default_on_not_found(Ok("Hello!")).unwrap(), "Hello!");
    }
}
