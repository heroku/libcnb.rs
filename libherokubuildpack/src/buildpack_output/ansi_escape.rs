/// Wraps each line in an ANSI escape sequence while preserving prior ANSI escape sequences.
///
/// ## Why does this exist?
///
/// When buildpack output is streamed to the user, each line is prefixed with `remote: ` by Git.
/// Any colorization of text will apply to those prefixes which is not the desired behavior. This
/// function colors lines of text while ensuring that styles are disabled at the end of each line.
///
/// ## Supports recursive colorization
///
/// Strings that are previously colorized will not be overridden by this function. For example,
/// if a word is already colored yellow, that word will continue to be yellow.
pub(crate) fn wrap_ansi_escape_each_line(ansi: &ANSI, body: impl AsRef<str>) -> String {
    let ansi_escape = ansi.to_str();
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

const RESET: &str = "\x1B[0m";
const RED: &str = "\x1B[0;31m";
const YELLOW: &str = "\x1B[0;33m";
const BOLD_CYAN: &str = "\x1B[1;36m";
const BOLD_PURPLE: &str = "\x1B[1;35m";
const DIM: &str = "\x1B[2;1m"; // Default color but softer/less vibrant

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub(crate) enum ANSI {
    Dim,
    Red,
    Yellow,
    BoldCyan,
    BoldPurple,
}

impl ANSI {
    fn to_str(&self) -> &'static str {
        match self {
            ANSI::Dim => DIM,
            ANSI::Red => RED,
            ANSI::Yellow => YELLOW,
            ANSI::BoldCyan => BOLD_CYAN,
            ANSI::BoldPurple => BOLD_PURPLE,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_line() {
        let actual = wrap_ansi_escape_each_line(&ANSI::Red, "\n");
        let expected = String::from("\n");
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_nested_color_at_start() {
        let start = wrap_ansi_escape_each_line(&ANSI::BoldCyan, "hello");
        let out = wrap_ansi_escape_each_line(&ANSI::Red, format!("{start} world"));
        let expected = format!("{RED}{BOLD_CYAN}hello{RESET}{RED} world{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_color_in_middle() {
        let middle = wrap_ansi_escape_each_line(&ANSI::BoldCyan, "middle");
        let out = wrap_ansi_escape_each_line(&ANSI::Red, format!("hello {middle} color"));
        let expected = format!("{RED}hello {BOLD_CYAN}middle{RESET}{RED} color{RESET}");
        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_color_at_end() {
        let end = wrap_ansi_escape_each_line(&ANSI::BoldCyan, "world");
        let out = wrap_ansi_escape_each_line(&ANSI::Red, format!("hello {end}"));
        let expected = format!("{RED}hello {BOLD_CYAN}world{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_double_nested_color() {
        let inner = wrap_ansi_escape_each_line(&ANSI::BoldCyan, "inner");
        let outer = wrap_ansi_escape_each_line(&ANSI::Red, format!("outer {inner}"));
        let out = wrap_ansi_escape_each_line(&ANSI::Yellow, format!("hello {outer}"));
        let expected = format!("{YELLOW}hello {RED}outer {BOLD_CYAN}inner{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn splits_newlines() {
        let actual = wrap_ansi_escape_each_line(&ANSI::Red, "hello\nworld");
        let expected = format!("{RED}hello{RESET}\n{RED}world{RESET}");

        assert_eq!(expected, actual);
    }

    #[test]
    fn simple_case() {
        let actual = wrap_ansi_escape_each_line(&ANSI::Red, "hello world");
        assert_eq!(format!("{RED}hello world{RESET}"), actual);
    }
}
