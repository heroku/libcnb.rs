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
                r"assertion failed: `(left contains right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`",
                $left,
                $left,
                $right,
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        if !$left.contains($right) {
            ::std::panic!(
                r"assertion failed: `(left contains right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}",
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
                r"assertion failed: `(left does not contain right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`",
                $left,
                $left,
                $right,
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        if $left.contains($right) {
            ::std::panic!(
                r"assertion failed: `(left does not contain right)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}",
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
                r"assertion failed: `(is empty)`
value (unescaped):
{}

value (escaped): `{:?}`",
                $value,
                $value,
            )
        }
    }};

    ($value:expr, $($arg:tt)+) => {{
        if !$value.is_empty() {
            ::std::panic!(
                r"assertion failed: `(is empty)`
value (unescaped):
{}

value (escaped): `{:?}`: {}",
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
        let regex = regex::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if !regex.is_match(&$left) {
            ::std::panic!(
                r"assertion failed: `(left matches right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`",
                $left,
                $left,
                regex
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let regex = regex::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if !regex.is_match(&$left) {
            ::std::panic!(
                r"assertion failed: `(left matches right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}",
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
/// assert_not_contains_match!(output, "^Test!");
/// ```
#[macro_export]
macro_rules! assert_not_contains_match {
    ($left:expr, $right:expr $(,)?) => {{
        let regex = regex::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if regex.is_match(&$left) {
            ::std::panic!(
                r"assertion failed: `(left does not match right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`",
                $left,
                $left,
                regex
            )
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let regex = regex::Regex::new(&format!("(?m){}", $right)).expect("should be a valid regex");
        if regex.is_match(&$left) {
            ::std::panic!(
                r"assertion failed: `(left does not match right pattern)`
left (unescaped):
{}

left (escaped): `{:?}`
right: `{:?}`: {}",
                $left,
                $left,
                regex,
                ::core::format_args!($($arg)+)
            )
        }
    }};
}

/// Asserts that an expression matches a given pattern.
///
/// This is a polyfill for `assert_matches!` to ensure 100% region coverage.
/// Unlike the crate or standard macros, this implementation is stable and avoids
/// generating unreachable panic branches that `llvm-cov` flags as uncovered regions.
///
/// Additionally, for expressions with a guard, this macro improves test error output
/// by explicitly printing the pattern, guard condition, and actual value on failure.
///
/// # Example
///
/// ```
/// use libcnb_test::assert_matches;
///
/// let result: Result<i32, String> = Ok(42);
/// assert_matches!(result, Ok(x) if x > 40);
/// ```
#[macro_export]
macro_rules! assert_matches {
    // With a guard (e.g. `Ok(x) if x > 10`)
    ($expression:expr, $pattern:pat if $guard:expr $(,)?) => {
        match $expression {
            $pattern if $guard => {}
            ref _actual => {
                ::std::panic!(
                    "Expected match pattern: {} where {}, but got {:?}",
                    stringify!($pattern),
                    stringify!($guard),
                    _actual
                );
            }
        }
    };

    // Without a guard (injects `if true` to force branch coverage)
    ($expression:expr, $pattern:pat $(,)?) => {
        match $expression {
            $pattern if true => {}
            ref _actual => {
                ::std::panic!(
                    "Expected match pattern: {}, but got {:?}",
                    stringify!($pattern),
                    _actual
                );
            }
        }
    };
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
    #[should_panic(expected = "should be a valid regex: Syntax(
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
regex parse error:
    (?m)(unclosed group
        ^
error: unclosed group
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
)")]
    fn contains_match_with_invalid_regex() {
        assert_contains_match!("Hello World!", "(unclosed group");
    }

    #[test]
    #[should_panic(expected = "should be a valid regex: Syntax(
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
regex parse error:
    (?m)(unclosed group
        ^
error: unclosed group
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
)")]
    fn contains_match_with_invalid_regex_and_args() {
        assert_contains_match!("Hello World!", "(unclosed group", "This should fail.");
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
    #[should_panic(expected = "should be a valid regex: Syntax(
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
regex parse error:
    (?m)(unclosed group
        ^
error: unclosed group
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
)")]
    fn not_contains_match_with_invalid_regex() {
        assert_not_contains_match!("Hello World!", "(unclosed group");
    }

    #[test]
    #[should_panic(expected = "should be a valid regex: Syntax(
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
regex parse error:
    (?m)(unclosed group
        ^
error: unclosed group
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
)")]
    fn not_contains_match_with_invalid_regex_and_args() {
        assert_not_contains_match!("Hello World!", "(unclosed group", "This will fail");
    }

    #[test]
    fn assert_matches_ok_simple() {
        let result: Result<i32, String> = Ok(42);
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn assert_matches_ok_with_guard() {
        let result: Result<i32, String> = Ok(42);
        assert_matches!(result, Ok(x) if x > 40);
    }

    #[test]
    fn assert_matches_err_simple() {
        let result: Result<i32, String> = Err("error".to_string());
        assert_matches!(result, Err(_));
    }

    #[test]
    #[should_panic(expected = "Expected match pattern: Ok(_), but got Err(\"error\")")]
    fn assert_matches_failure_simple() {
        let result: Result<i32, String> = Err("error".to_string());
        assert_matches!(result, Ok(_));
    }

    #[test]
    #[should_panic(expected = "Expected match pattern: Ok(x) where x > 50, but got Ok(42)")]
    fn assert_matches_failure_with_guard() {
        let result: Result<i32, String> = Ok(42);
        assert_matches!(result, Ok(x) if x > 50);
    }

    #[test]
    fn assert_matches_option_some() {
        let value: Option<&str> = Some("hello");
        assert_matches!(value, Some("hello"));
    }

    #[test]
    fn assert_matches_option_none() {
        let value: Option<&str> = None;
        assert_matches!(value, None);
    }
}
