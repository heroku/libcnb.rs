/// Asserts that `left` contains `right`.
///
/// Commonly used when asserting `pack` output in integration tests. Expands to a [`str::contains`]
/// call and logs `left` (in unescaped and escaped form) as well as `right` on failure.
///
/// # Example
///
/// ```
/// use libcnb_test::assert_contains;
///
/// let output = "Hello World!\nHello Integration Test!";
/// assert_contains!(output, "Integration");
/// ```
#[macro_export]
macro_rules! assert_contains {
    ($left:expr, $right:expr $(,)?) => {{
        if !$left.contains($right) {
            ::std::panic!(
                r#"assertion failed: `(left contains right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`"#,
                $left,
                $left,
                $right,
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        if !$left.contains($right) {
            ::std::panic!(
                r#"assertion failed: `(left contains right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}"#,
                $left,
                $left,
                $right,
                ::core::format_args!($($arg)+)
            )
        }
    }};
}

/// Asserts that `left` does not contain `right`.
///
/// Commonly used when asserting `pack` output in integration tests. Expands to a [`str::contains`]
/// call and logs `left` (in unescaped and escaped form) as well as `right` on failure.
///
/// # Example
///
/// ```
/// use libcnb_test::assert_not_contains;
///
/// let output = "Hello World!\nHello Integration Test!";
/// assert_not_contains!(output, "Bahamas");
/// ```
#[macro_export]
macro_rules! assert_not_contains {
    ($left:expr, $right:expr $(,)?) => {{
        if $left.contains($right) {
            ::std::panic!(
                r#"assertion failed: `(left does not contain right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`"#,
                $left,
                $left,
                $right,
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        if $left.contains($right) {
            ::std::panic!(
                r#"assertion failed: `(left does not contain right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}"#,
                $left,
                $left,
                $right,
                ::core::format_args!($($arg)+)
            )
        }
    }};
}

/// Asserts that the provided value is empty.
///
/// Commonly used when asserting `pack` output in integration tests. Expands to a [`str::is_empty`]
/// call and logs the value (in unescaped and escaped form) on failure.
///
/// # Example
///
/// ```
/// use libcnb_test::assert_empty;
///
/// let output = "";
/// assert_empty!(output);
/// ```
#[macro_export]
macro_rules! assert_empty {
    ($value:expr $(,)?) => {{
        if !$value.is_empty() {
            ::std::panic!(
                r#"assertion failed: `(is empty)`
value (unescaped):
{}

value (escaped): `{:?}`"#,
                $value,
                $value,
            )
        }
    }};

    ($value:expr, $($arg:tt)+) => {{
        if !$value.is_empty() {
            ::std::panic!(
                r#"assertion failed: `(is empty)`
value (unescaped):
{}

value (escaped): `{:?}`: {}"#,
                $value,
                $value,
                ::core::format_args!($($arg)+)
            )
        }
    }};
}

/// Asserts that `left` contains the `right` pattern (regular expression).
///
/// Commonly used when asserting `pack` output in integration tests. Expands to a regular
/// expression match test and logs `left` (in unescaped and escaped form) as well as `right`
/// on failure.
///
/// Multi-line mode is automatically enabled on regular expressions. If this is not what you
/// want it can be disabled by adding `(?-m)` to the start of your pattern.
///
/// # Example
///
/// ```
/// use libcnb_test::assert_contains_match;
///
/// let output = "Hello World!\nHello Integration Test!";
/// assert_contains_match!(output, "Test!$");
/// ```
#[macro_export]
macro_rules! assert_contains_match {
    ($left:expr, $right:expr $(,)?) => {{
        let regex = regex_lite::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if !regex.is_match(&$left) {
            ::std::panic!(
                r#"assertion failed: `(left matches right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`"#,
                $left,
                $left,
                regex
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let regex = regex_lite::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if !regex.is_match(&$left) {
            ::std::panic!(
                r#"assertion failed: `(left matches right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}"#,
                $left,
                $left,
                regex,
                ::core::format_args!($($arg)+)
            )
        }
    }};
}

/// Asserts that `left` does not contain the `right` pattern (regular expression).
///
/// Commonly used when asserting `pack` output in integration tests. Expands to a regular
/// expression match test and logs `left` (in unescaped and escaped form) as well as `right`
/// on failure.
///
/// Multi-line mode is automatically enabled on regular expressions. If this is not what you
/// want it can be disabled by adding `(?-m)` to the start of your pattern.
///
/// # Example
///
/// ```
/// use libcnb_test::assert_not_contains_match;
///
/// let output = "Hello World!\nHello Integration Test!";
/// assert_not_contains_match!(output, "Test!$");
/// ```
#[macro_export]
macro_rules! assert_not_contains_match {
    ($left:expr, $right:expr $(,)?) => {{
        let regex = regex_lite::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if regex.is_match(&$left) {
            ::std::panic!(
                r#"assertion failed: `(left does not match right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`"#,
                $left,
                $left,
                regex
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let regex = regex_lite::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if regex.is_match(&$left) {
            ::std::panic!(
                r#"assertion failed: `(left does not match right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}"#,
                $left,
                $left,
                regex,
                ::core::format_args!($($arg)+)
            )
        }
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn contains_simple() {
        assert_contains!("Hello World!", "World");
    }

    #[test]
    fn contains_simple_with_args() {
        assert_contains!("Hello World!", "World", "World must be greeted!");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left contains right)`
left (unescaped):
foo

left (escaped): `\"foo\"`
right: `\"bar\"`")]
    fn contains_simple_failure() {
        assert_contains!("foo", "bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left contains right)`
left (unescaped):
Hello Germany!

left (escaped): `\"Hello Germany!\"`
right: `\"World\"`: World must be greeted!")]
    fn contains_simple_failure_with_args() {
        assert_contains!("Hello Germany!", "World", "World must be greeted!");
    }

    #[test]
    fn contains_multiline() {
        assert_contains!("Hello World!\nFoo\nBar\nBaz", "Bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left contains right)`
left (unescaped):
Hello World!
Foo
Bar
Baz

left (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`
right: `\"Eggs\"`")]
    fn contains_multiline_failure() {
        assert_contains!("Hello World!\nFoo\nBar\nBaz", "Eggs");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left contains right)`
left (unescaped):
Hello World!
Foo
Bar
Baz

left (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`
right: `\"Eggs\"`: We need eggs!")]
    fn contains_multiline_failure_with_args() {
        assert_contains!("Hello World!\nFoo\nBar\nBaz", "Eggs", "We need eggs!");
    }

    #[test]
    fn not_contains_simple() {
        assert_not_contains!("Hello World!", "Bahamas");
    }

    #[test]
    fn not_contains_simple_with_args() {
        assert_not_contains!("Hello World!", "Bahamas", "Bahamas must not be greeted!");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not contain right)`
left (unescaped):
foobar

left (escaped): `\"foobar\"`
right: `\"bar\"`")]
    fn not_contains_simple_failure() {
        assert_not_contains!("foobar", "bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not contain right)`
left (unescaped):
Hello Germany!

left (escaped): `\"Hello Germany!\"`
right: `\"Germany\"`: Germany must be greeted!")]
    fn not_contains_simple_failure_with_args() {
        assert_not_contains!("Hello Germany!", "Germany", "Germany must be greeted!");
    }

    #[test]
    fn not_contains_multiline() {
        assert_not_contains!("Hello World!\nFoo\nBar\nBaz", "Germany");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not contain right)`
left (unescaped):
Hello World!
Foo
Bar
Baz

left (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`
right: `\"Bar\"`")]
    fn not_contains_multiline_failure() {
        assert_not_contains!("Hello World!\nFoo\nBar\nBaz", "Bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not contain right)`
left (unescaped):
Hello Eggs!
Foo
Bar
Baz

left (escaped): `\"Hello Eggs!\\nFoo\\nBar\\nBaz\"`
right: `\"Eggs\"`: We must not have eggs!")]
    fn not_contains_multiline_failure_with_args() {
        assert_not_contains!(
            "Hello Eggs!\nFoo\nBar\nBaz",
            "Eggs",
            "We must not have eggs!"
        );
    }

    #[test]
    fn empty_simple() {
        assert_empty!("");
    }

    #[test]
    fn empty_simple_with_args() {
        assert_empty!("", "Value must be empty!");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(is empty)`
value (unescaped):
foo

value (escaped): `\"foo\"`")]
    fn empty_simple_failure() {
        assert_empty!("foo");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(is empty)`
value (unescaped):
Hello World!

value (escaped): `\"Hello World!\"`: Greeting must be empty!")]
    fn empty_simple_failure_with_args() {
        assert_empty!("Hello World!", "Greeting must be empty!");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(is empty)`
value (unescaped):
Hello World!
Foo
Bar
Baz

value (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`")]
    fn empty_multiline_failure() {
        assert_empty!("Hello World!\nFoo\nBar\nBaz");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(is empty)`
value (unescaped):
Hello World!
Foo
Bar
Baz

value (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`: Greeting must be empty!")]
    fn empty_multiline_failure_with_args() {
        assert_empty!("Hello World!\nFoo\nBar\nBaz", "Greeting must be empty!");
    }

    #[test]
    fn contains_match_simple() {
        assert_contains_match!("Hello World!", "(?i)hello world!");
    }

    #[test]
    fn contains_match_simple_with_args() {
        assert_contains_match!("Hello World!", "(?i)hello world!", "World must be greeted");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left matches right pattern)`
left (unescaped):
foo

left (escaped): `\"foo\"`
right: `Regex(\"(?m)bar\")`")]
    fn contains_match_simple_failure() {
        assert_contains_match!("foo", "bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left matches right pattern)`
left (unescaped):
Hello World!

left (escaped): `\"Hello World!\"`
right: `Regex(\"(?m)(?-i)world\")`: World must be case-sensitively greeted!")]
    fn contains_match_simple_failure_with_args() {
        assert_contains_match!(
            "Hello World!",
            "(?-i)world",
            "World must be case-sensitively greeted!"
        );
    }

    #[test]
    fn contains_match_multiline() {
        assert_contains_match!("Hello World!\nFoo\nBar\nBaz", "^Bar$");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left matches right pattern)`
left (unescaped):
Hello World!
Foo
Bar
Baz

left (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`
right: `Regex(\"(?m)Eggs\")`")]
    fn contains_match_multiline_failure() {
        assert_contains_match!("Hello World!\nFoo\nBar\nBaz", "Eggs");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left matches right pattern)`
left (unescaped):
Hello World!
Foo
Bar
Baz

left (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`
right: `Regex(\"(?m)Eggs\")`: We need eggs!")]
    fn contains_match_multiline_failure_with_args() {
        assert_contains_match!("Hello World!\nFoo\nBar\nBaz", "Eggs", "We need eggs!");
    }

    #[test]
    #[should_panic(
        expected = "should be a valid regex: Error { msg: \"found open group without closing ')'\" }"
    )]
    fn contains_match_with_invalid_regex() {
        assert_contains_match!("Hello World!", "(unclosed capture");
    }

    #[test]
    #[should_panic(
        expected = "should be a valid regex: Error { msg: \"found open group without closing ')'\" }"
    )]
    fn contains_match_with_invalid_regex_and_args() {
        assert_contains_match!("Hello World!", "(unclosed capture", "This should fail.");
    }

    #[test]
    fn not_contains_match_simple() {
        assert_not_contains_match!("Hello World!", "^World");
    }

    #[test]
    fn not_contains_match_simple_with_args() {
        assert_not_contains_match!("Hello World!", "^World", "World must not be at the start!");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not match right pattern)`
left (unescaped):
foobar

left (escaped): `\"foobar\"`
right: `Regex(\"(?m)bar\")`")]
    fn not_contains_match_simple_failure() {
        assert_not_contains_match!("foobar", "bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not match right pattern)`
left (unescaped):
Hello Germany!

left (escaped): `\"Hello Germany!\"`
right: `Regex(\"(?m)Germany!$\")`: Germany must not be greeted!")]
    fn not_contains_match_simple_failure_with_args() {
        assert_not_contains_match!(
            "Hello Germany!",
            "Germany!$",
            "Germany must not be greeted!"
        );
    }

    #[test]
    fn not_contains_match_multiline() {
        assert_not_contains_match!("Hello World!\nFoo\nBar\nBaz", "^Germany$");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not match right pattern)`
left (unescaped):
Hello World!
Foo
Bar
Baz

left (escaped): `\"Hello World!\\nFoo\\nBar\\nBaz\"`
right: `Regex(\"(?m)^Bar$\")`")]
    fn not_contains_match_multiline_failure() {
        assert_not_contains_match!("Hello World!\nFoo\nBar\nBaz", "^Bar$");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left does not match right pattern)`
left (unescaped):
Hello Eggs!
Foo
Bar
Baz

left (escaped): `\"Hello Eggs!\\nFoo\\nBar\\nBaz\"`
right: `Regex(\"(?m)Eggs!$\")`: We must not have eggs!")]
    fn not_contains_match_multiline_failure_with_args() {
        assert_not_contains_match!(
            "Hello Eggs!\nFoo\nBar\nBaz",
            "Eggs!$",
            "We must not have eggs!"
        );
    }

    #[test]
    #[should_panic(
        expected = "should be a valid regex: Error { msg: \"found open group without closing ')'\" }"
    )]
    fn not_contains_match_with_invalid_regex() {
        assert_not_contains_match!("Hello World!", "(unclosed capture");
    }

    #[test]
    #[should_panic(
        expected = "should be a valid regex: Error { msg: \"found open group without closing ')'\" }"
    )]
    fn not_contains_match_with_invalid_regex_and_args() {
        assert_not_contains_match!("Hello World!", "(unclosed capture", "This will fail");
    }
}
