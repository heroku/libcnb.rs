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
