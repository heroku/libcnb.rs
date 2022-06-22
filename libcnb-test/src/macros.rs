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
/// assert_not_contains!(output, "Integration");
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
}
