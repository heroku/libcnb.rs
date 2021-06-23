use std::error::Error;

use std::process::exit;

use serde::de::DeserializeOwned;

use crate::build::{cnb_runtime_build, BuildContext};
use crate::detect::{cnb_runtime_detect, DetectContext, DetectResult};
use crate::error::LibCnbErrorHandle;
use crate::{
    platform::Platform,
    LibCnbError,
};

#[cfg(any(target_family = "unix"))]
pub fn cnb_runtime<P: Platform, BM: DeserializeOwned, E: Error>(
    detect_fn: impl Fn(DetectContext<P, BM>) -> Result<DetectResult, LibCnbError<E>>,
    build_fn: impl Fn(BuildContext<P, BM>) -> Result<(), LibCnbError<E>>,
    error_handler: impl LibCnbErrorHandle<E>,
) {
    let current_exe = std::env::current_exe().ok();
    let current_exe_file_name = current_exe
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|file_name| file_name.to_str());

    let result = match current_exe_file_name {
        Some("detect") => cnb_runtime_detect(detect_fn),
        Some("build") => cnb_runtime_build(build_fn),
        Some(_) | None => exit(255),
    };

    if let Err(lib_cnb_error) = result {
        error_handler.handle_error(lib_cnb_error);
        exit(123);
    }
}
