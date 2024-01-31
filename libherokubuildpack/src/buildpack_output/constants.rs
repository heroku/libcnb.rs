pub(crate) const RED: &str = "\x1B[0;31m";
pub(crate) const YELLOW: &str = "\x1B[0;33m";
pub(crate) const CYAN: &str = "\x1B[0;36m";

pub(crate) const BOLD_CYAN: &str = "\x1B[1;36m";
pub(crate) const BOLD_PURPLE: &str = "\x1B[1;35m"; // Magenta

#[cfg(test)]
pub(crate) const DEFAULT_DIM: &str = "\x1B[2;1m"; // Default color but softer/less vibrant
pub(crate) const RESET: &str = "\x1B[0m";

#[cfg(test)]
pub(crate) const NO_COLOR: &str = "\x1B[1;39m"; // Differentiate between color clear and explicit no color https://github.com/heroku/buildpacks-ruby/pull/155#discussion_r1260029915
#[cfg(test)]
pub(crate) const ALL_CODES: [&str; 7] = [
    RED,
    YELLOW,
    CYAN,
    BOLD_CYAN,
    BOLD_PURPLE,
    DEFAULT_DIM,
    RESET,
];
