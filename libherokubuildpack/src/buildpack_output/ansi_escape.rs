/// Colorizes a body while preserving existing color/reset combinations and clearing before newlines.
///
/// Colors with newlines are a problem since the contents stream to git which prepends `remote:` before the `libcnb_test`
/// if we don't clear, then we will colorize output that isn't ours.
///
/// Explicitly uncolored output is handled by treating `\x1b[1;39m` (`NO_COLOR`) as a distinct case from `\x1b[0m`.
pub(crate) fn colorize_multiline(color: &str, body: impl AsRef<str>) -> String {
    body.as_ref()
        .split('\n')
        // If sub contents are colorized it will contain SUBCOLOR ... RESET. After the reset,
        // ensure we change back to the current color
        .map(|line| line.replace(RESET, &format!("{RESET}{color}"))) // Handles nested color
        // Set the main color for each line and reset after so we don't colorize `remote:` by accident
        .map(|line| format!("{color}{line}{RESET}"))
        // The above logic causes redundant colors and resets, clean them up
        .map(|line| line.replace(&format!("{RESET}{color}{RESET}"), RESET))
        .map(|line| line.replace(&format!("{color}{color}"), color)) // Reduce useless color
        .map(|line| line.replace(&format!("{color}{RESET}"), "")) // Do not colorize empty lines
        .collect::<Vec<String>>()
        .join("\n")
}

pub(crate) const RED: &str = "\x1B[0;31m";
pub(crate) const YELLOW: &str = "\x1B[0;33m";
pub(crate) const CYAN: &str = "\x1B[0;36m";

pub(crate) const BOLD_CYAN: &str = "\x1B[1;36m";
pub(crate) const BOLD_PURPLE: &str = "\x1B[1;35m"; // Magenta

pub(crate) const RESET: &str = "\x1B[0m";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn handles_explicitly_removed_colors() {
        // Differentiate between color clear and explicit no color https://github.com/heroku/buildpacks-ruby/pull/155#discussion_r1260029915
        const NO_COLOR: &str = "\x1B[1;39m";
        let nested = colorize_multiline(NO_COLOR, "nested");

        let out = colorize_multiline(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {NO_COLOR}nested{RESET}{RED} color{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_colors() {
        let nested = colorize_multiline(CYAN, "nested");

        let out = colorize_multiline(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {CYAN}nested{RESET}{RED} color{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn splits_newlines() {
        let actual = colorize_multiline(RED, "hello\nworld");
        let expected = format!("{RED}hello{RESET}\n{RED}world{RESET}");

        assert_eq!(expected, actual);
    }

    #[test]
    fn simple_case() {
        let actual = colorize_multiline(RED, "hello world");
        assert_eq!(format!("{RED}hello world{RESET}"), actual);
    }
}
