/// Smartly injects an ANSI escape sequence as the default into the given string.
///
/// All sub sequences of the given string that are not preceded by an ANSI escape sequence other than reset will use
/// the given ANSI escape sequence as the default.
///
/// The given string is allowed to already contain ANSI sequences which will not be overridden by this function. For
/// example, this function can be used to color all text red, but if a word is already colored yellow, that word will
/// continue to be yellow.
///
/// The given ANSI escape sequence will in injected into each line of the given string separately, followed by a reset
/// at the end of each line. This ensure that any downstream consumers of the resulting string can process it
/// line-by-line without losing context. One example is the `remote: ` prefix that Git adds when streaming output from
/// a buildpack.
pub(crate) fn inject_default_ansi_escape(ansi_escape: &str, body: impl AsRef<str>) -> String {
    body.as_ref()
        .split('\n')
        // If sub contents are colorized it will contain SUBCOLOR ... RESET. After the reset,
        // ensure we change back to the current color
        .map(|line| line.replace(RESET, &format!("{RESET}{ansi_escape}"))) // Handles nested color
        // Set the main color for each line and reset after so we don't colorize `remote:` by accident
        .map(|line| format!("{ansi_escape}{line}{RESET}"))
        // The above logic causes redundant colors and resets, clean them up
        .map(|line| line.replace(&format!("{RESET}{ansi_escape}{RESET}"), RESET))
        .map(|line| line.replace(&format!("{ansi_escape}{ansi_escape}"), ansi_escape)) // Reduce useless color
        .map(|line| line.replace(&format!("{ansi_escape}{RESET}"), "")) // Do not colorize empty lines
        .collect::<Vec<String>>()
        .join("\n")
}

pub(crate) const RESET: &str = "\x1B[0m";

pub(crate) const RED: &str = "\x1B[0;31m";
pub(crate) const YELLOW: &str = "\x1B[0;33m";
pub(crate) const CYAN: &str = "\x1B[0;36m";

pub(crate) const BOLD_CYAN: &str = "\x1B[1;36m";
pub(crate) const BOLD_PURPLE: &str = "\x1B[1;35m"; // Magenta

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn handles_explicitly_removed_colors() {
        // Differentiate between color clear and explicit no color https://github.com/heroku/buildpacks-ruby/pull/155#discussion_r1260029915
        const NO_COLOR: &str = "\x1B[1;39m";
        let nested = inject_default_ansi_escape(NO_COLOR, "nested");

        let out = inject_default_ansi_escape(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {NO_COLOR}nested{RESET}{RED} color{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_colors() {
        let nested = inject_default_ansi_escape(CYAN, "nested");

        let out = inject_default_ansi_escape(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {CYAN}nested{RESET}{RED} color{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn splits_newlines() {
        let actual = inject_default_ansi_escape(RED, "hello\nworld");
        let expected = format!("{RED}hello{RESET}\n{RED}world{RESET}");

        assert_eq!(expected, actual);
    }

    #[test]
    fn simple_case() {
        let actual = inject_default_ansi_escape(RED, "hello world");
        assert_eq!(format!("{RED}hello world{RESET}"), actual);
    }
}
