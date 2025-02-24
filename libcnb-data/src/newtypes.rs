/// Macro to generate a newtype backed by `String` that is validated by a regular expression.
///
/// Automatically implements the following traits for the newtype:
/// - [`Clone`]
/// - [`Debug`]
/// - [`Display`](std::fmt::Display)
/// - [`Eq`]
/// - [`Hash`]
/// - [`Ord`]
/// - [`PartialEq`]
/// - [`PartialOrd`]
/// - [`serde::Deserialize`]
/// - [`serde::Serialize`]
/// - [`FromStr`](std::str::FromStr)
/// - [`Borrow<String>`](std::borrow::Borrow<String>)
/// - [`Borrow<str>`](std::borrow::Borrow<str>)
/// - [`Deref<Target=String>`]
/// - [`AsRef<String>`]
///
/// This macro also generates another macro that can be used to construct values of the newtype from
/// literal strings at compile time. Compilation will fail if such a macro is used with a string
/// that is not valid for the corresponding newtype. This removes the need for explicit `unwrap`
/// calls that might fail at runtime.
///
/// # Usage:
// This has to use compile_fail since `libcnb_newtype` is not public.
/// ```compile_fail
/// use crate::newtypes::libcnb_newtype;
///
/// libcnb_newtype!(
///     // The module of this crate that exports the newtype publicly. Since it might differ from
///     // the actual module structure, the macro needs a way to determine how to import the type
///     // from a user's buildpack crate.
///     buildpack
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
        #[derive(Debug, Eq, PartialEq, ::serde::Serialize, Clone, Hash)]
        $(#[$type_attributes])*
        #[allow(unreachable_pub)]
        pub struct $name(String);

        #[derive(::thiserror::Error, Debug, Eq, PartialEq)]
        $(#[$error_type_attributes])*
        #[allow(unreachable_pub)]
        pub enum $error_name {
            InvalidValue(String),
        }

        impl ::std::fmt::Display for $error_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                match self {
                    Self::InvalidValue(value) => {
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

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                String::deserialize(d)?
                    .parse::<$name>()
                    .map_err(::serde::de::Error::custom)
            }
        }

        impl ::std::borrow::Borrow<String> for $name {
            fn borrow(&self) -> &String {
                &self.0
            }
        }

        impl ::std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
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

        impl ::std::cmp::Ord for $name {
           fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl ::std::cmp::PartialOrd for $name {
           fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
                Some(self.cmp(&other))
            }
        }

        impl $name {
            /// Construct an instance of this type without performing validation.
            ///
            /// This should not be used directly, and is only public so that it
            /// can be used by the compile-time validation macro.
            #[must_use]
            #[doc(hidden)]
            #[allow(unreachable_pub)]
            pub fn new_unchecked(value: &str) -> Self {
                Self(String::from(value))
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
                        base::$name::new_unchecked($value)
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
mod tests {
    use serde_test::{Token, assert_de_tokens, assert_de_tokens_error};

    libcnb_newtype!(
        newtypes::tests,
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
    fn type_eq() {
        assert_eq!(
            "Katrin".parse::<CapitalizedName>(),
            "Katrin".parse::<CapitalizedName>()
        );
        assert_ne!(
            "Katrin".parse::<CapitalizedName>(),
            "Manuel".parse::<CapitalizedName>()
        );
    }

    #[test]
    fn literal_macro_success() {
        assert_eq!("Jonas", capitalized_name!("Jonas").as_ref());
    }

    #[test]
    fn deref() {
        fn foo(name: &str) {
            assert_eq!(name, "Johanna");
        }

        let name = "Johanna".parse::<CapitalizedName>().unwrap();
        foo(&name);
    }

    #[test]
    fn join() {
        let names = [capitalized_name!("A"), capitalized_name!("B")];
        assert_eq!("A, B", names.join(", "));
    }

    #[test]
    fn ord() {
        let mut names = [
            capitalized_name!("A"),
            capitalized_name!("C"),
            capitalized_name!("B"),
        ];
        names.sort();

        assert_eq!(
            [
                capitalized_name!("A"),
                capitalized_name!("B"),
                capitalized_name!("C")
            ],
            names
        );
    }

    #[test]
    fn deserialize() {
        assert_de_tokens(&capitalized_name!("Jonas"), &[Token::Str("Jonas")]);
        assert_de_tokens(&capitalized_name!("Johanna"), &[Token::Str("Johanna")]);

        assert_de_tokens_error::<CapitalizedName>(&[Token::Str("Manuel")], "Invalid Value: Manuel");
        assert_de_tokens_error::<CapitalizedName>(&[Token::Str("katrin")], "Invalid Value: katrin");
    }
}
