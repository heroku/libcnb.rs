use crate::LibCnbError;
use std::error::Error;

pub trait LibCnbErrorHandle<E: Error> {
    fn handle_error(&self, error: LibCnbError<E>);
}

pub trait BuildpackErrorHandle<E> {
    fn handle_error(&self, error: E);
}

impl<T: BuildpackErrorHandle<E>, E: Error> LibCnbErrorHandle<E> for T {
    fn handle_error(&self, error: LibCnbError<E>) {
        match error {
            LibCnbError::BuildpackError(buildpack_error) => self.handle_error(buildpack_error),
            LibCnbError::LayerLifecycleError(_) => {}
            LibCnbError::ProcessTypeError(_) => {}
        }
    }
}
