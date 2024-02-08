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
        .map(|line| line.replace(&format!("{ansi_escape}{ansi_escape}"), ansi_escape)) // Reduce useless color
        .map(|line| line.replace(&format!("{ansi_escape}{RESET}"), "")) // Empty lines or where the nested color is at the end of the line
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
    fn empty_line() {
        let actual = inject_default_ansi_escape(RED, "\n");
        let expected = String::from("\n");
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_nested_color_at_start() {
        let start = inject_default_ansi_escape(CYAN, "hello");
        let out = inject_default_ansi_escape(RED, format!("{start} world"));
        let expected = format!("{RED}{CYAN}hello{RESET}{RED} world{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_color_in_middle() {
        let middle = inject_default_ansi_escape(CYAN, "middle");
        let out = inject_default_ansi_escape(RED, format!("hello {middle} color"));
        let expected = format!("{RED}hello {CYAN}middle{RESET}{RED} color{RESET}");
        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_color_at_end() {
        let end = inject_default_ansi_escape(CYAN, "world");
        let out = inject_default_ansi_escape(RED, format!("hello {end}"));
        let expected = format!("{RED}hello {CYAN}world{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_double_nested_color() {
        let inner = inject_default_ansi_escape(CYAN, "inner");
        let outer = inject_default_ansi_escape(RED, format!("outer {inner}"));
        let out = inject_default_ansi_escape(YELLOW, format!("hello {outer}"));
        let expected = format!("{YELLOW}hello {RED}outer {CYAN}inner{RESET}");

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
