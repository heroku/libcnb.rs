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

#[cfg(test)]
mod test {
    #[test]
    fn simple() {
        assert_contains!("Hello World!", "World");
    }

    #[test]
    fn simple_with_args() {
        assert_contains!("Hello World!", "World", "World must be greeted!");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left contains right)`
left (unescaped):
foo

left (escaped): `\"foo\"`
right: `\"bar\"`")]
    fn simple_failure() {
        assert_contains!("foo", "bar");
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left contains right)`
left (unescaped):
Hello Germany!

left (escaped): `\"Hello Germany!\"`
right: `\"World\"`: World must be greeted!")]
    fn simple_failure_with_args() {
        assert_contains!("Hello Germany!", "World", "World must be greeted!");
    }

    #[test]
    fn multiline() {
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
    fn multiline_failure() {
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
    fn multiline_failure_with_args() {
        assert_contains!("Hello World!\nFoo\nBar\nBaz", "Eggs", "We need eggs!");
    }
}
