///! libcnb exit code constants
///
/// Constants are prefixed with the phase they're valid for since their meaning can change between
/// different CNB phases.

pub(crate) const GENERIC_SUCCESS: i32 = 0;
pub(crate) const GENERIC_UNSPECIFIED_ERROR: i32 = 1;
pub(crate) const GENERIC_CNB_API_MISMATCH_ERROR: i32 = 254;
pub(crate) const GENERIC_UNEXPECTED_EXECUTABLE_NAME_ERROR: i32 = 255;

pub(crate) const DETECT_DETECTION_PASSED: i32 = 0;
pub(crate) const DETECT_DETECTION_FAILED: i32 = 100;
