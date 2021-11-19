// The compiler has a hard time figuring out that we actually use libcnb_sys in here. To avoid
// false-positive warnings about unused crates, we import it as _ here:
use libcnb_proc_macros as _;

/// Macro to generate a newtype backed by `String` that is validated by a regular expression.
///
/// Automatically implements the following traits for the newtype:
/// - [`Debug`]
/// - [`Display`]
/// - [`Eq`]
/// - [`PartialEq`]
/// - [`serde::Deserialize`]
/// - [`serde::Serialize`]
/// - [`FromStr`]
/// - [`Borrow<String>`]
/// - [`Deref<Target=String>`]
/// - [`AsRef<String>`]
///
/// This macro also generates another macro that can be used to construct values of the newtype from
/// literal strings at compile time. Compilation will fail if such a macro is used with a string
/// that is not valid for the corresponding newtype. This removes the need for explicit `unwrap`
/// calls that might fail at runtime.
///
/// # Usage:
/// ```
/// libcnb_newtype!(
///     // The module of this crate that exports the newtype publicly. Since it might differ from
///     // the actual module structure, the macro needs a way to determine how to import the type
///     // from a user's buildpack crate.
///     tests::doctest
///     /// RustDoc for the macro (optional)
///     buildpack_id,
///     /// RustDoc for the newtype itself (optional)
///     BuildpackId,
///     /// RustDoc for the newtype error (optional)
///     BuildpackIdError,
///     // The regular expression that must match for the String to be valid. Uses the `fancy_regex`
///     // crate which supports negative lookarounds.
///     r"^[[:alnum:]./-]+$",
/// );
///
/// // Using the type:
/// let bp_id = "bar".parse::<BuildpackId>().unwrap();
///
/// // Using the macro for newtype literals with compile-type checks:
/// let bp_id = buildpack_id!("foo");
/// ```
macro_rules! libcnb_newtype {
    (
        $path:path,
        $(#[$macro_attributes:meta])*
        $macro_name:ident,
        $(#[$type_attributes:meta])*
        $name:ident,
        $(#[$error_type_attributes:meta])*
        $error_name:ident,
        $regex:expr
    ) => {
        #[derive(Debug, Eq, PartialEq, ::serde::Deserialize, ::serde::Serialize)]
        $(#[$type_attributes])*
        pub struct $name(String);

        #[derive(::thiserror::Error, Debug, Eq, PartialEq)]
        $(#[$error_type_attributes])*
        pub enum $error_name {
            InvalidValue(String),
        }

        impl ::std::fmt::Display for $error_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                match self {
                    $error_name::InvalidValue(value) => {
                        ::std::write!(f, "Invalid Value: {}", value)
                    }
                }
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $error_name;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let regex_matches = ::fancy_regex::Regex::new($regex)
                    .and_then(|regex| regex.is_match(value))
                    .unwrap_or(false);

                if regex_matches {
                    Ok(Self(String::from(value)))
                } else {
                    Err($error_name::InvalidValue(String::from(value)))
                }
            }
        }

        impl ::std::borrow::Borrow<String> for $name {
            fn borrow(&self) -> &String {
                &self.0
            }
        }

        impl ::std::ops::Deref for $name {
            type Target = String;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::convert::AsRef<String> for $name {
            fn as_ref(&self) -> &String {
                &self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                ::std::write!(f, "{}", self.0)
            }
        }

        #[macro_export]
        $(#[$macro_attributes])*
        macro_rules! $macro_name {
            ($value:expr) => {
                $crate::internals::verify_regex!(
                    $regex,
                    $value,
                    {
                        use $crate::$path as base;
                        $value.parse::<base::$name>().unwrap()
                    },
                    compile_error!(concat!(
                        stringify!($value),
                        " is not a valid ",
                        stringify!($name),
                        " value!"
                    ))
                )
            }
        }
    };
}

pub(crate) use libcnb_newtype;

#[cfg(test)]
mod test {
    use super::libcnb_newtype;

    libcnb_newtype!(
        newtypes::test,
        capitalized_name,
        CapitalizedName,
        CapitalizedNameError,
        r"^(?!Manuel$)[A-Z][a-z]*$"
    );

    #[test]
    fn test() {
        assert!("Katrin".parse::<CapitalizedName>().is_ok());

        assert_eq!(
            "manuel".parse::<CapitalizedName>(),
            Err(CapitalizedNameError::InvalidValue(String::from("manuel")))
        );

        assert_eq!(
            "Manuel".parse::<CapitalizedName>(),
            Err(CapitalizedNameError::InvalidValue(String::from("Manuel")))
        );
    }

    #[test]
    fn test_literal_macro_success() {
        assert_eq!("Jonas", capitalized_name!("Jonas").as_ref());
    }

    #[test]
    fn test_deref() {
        fn foo(name: &str) {
            assert_eq!(name, "Johanna");
        }

        let name = "Johanna".parse::<CapitalizedName>().unwrap();
        foo(&name);
    }
}
