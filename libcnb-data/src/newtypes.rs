/// Macro to generate newtypes backed by `String`
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
/// # Usage:
/// ```
/// libcnb_newtype!(
///     /// RustDoc for the newtype itself (optional)
///     BuildpackId,
///     /// RustDoc for the newtype error (optional)
///     BuildpackIdError,
///     // The regular expression that must match for the String to be valid
///     r"^[[:alnum:]./-]+$",
///     // Additional predicate function to do further validation (optional)
///     |id| { id != "app" && id != "config" }
/// );
/// ```
macro_rules! libcnb_newtype {
    (
        $(#[$type_attributes:meta])*
        $name:ident,
        $(#[$error_type_attributes:meta])*
        $error_name:ident,
        $regex:expr
    ) => {
            libcnb_newtype!(
                $(#[$type_attributes])*
                $name,
                $(#[$error_type_attributes])*
                $error_name,
                $regex,
                |_| true
            );
    };
    (
        $(#[$type_attributes:meta])*
        $name:ident,
        $(#[$error_type_attributes:meta])*
        $error_name:ident,
        $regex:expr,
        $extra_predicate:expr
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
                let regex_matches = ::regex::Regex::new($regex).unwrap().is_match(value);
                let predicate_matches = $extra_predicate(value);

                if regex_matches && predicate_matches {
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
    };
}

pub(crate) use libcnb_newtype;

#[cfg(test)]
mod test {
    use super::libcnb_newtype;

    #[test]
    fn test() {
        libcnb_newtype!(CapitalizedName, CapitalizedNameError, r"^[A-Z][a-z]*$");

        assert!("Manuel".parse::<CapitalizedName>().is_ok());

        assert_eq!(
            "manuel".parse::<CapitalizedName>(),
            Err(CapitalizedNameError::InvalidValue(String::from("manuel")))
        );
    }

    #[test]
    fn test_extra_predicate() {
        libcnb_newtype!(
            CapitalizedName,
            CapitalizedNameError,
            r"^[A-Z][a-z]*$",
            |value| value != "Manuel"
        );

        assert!("Jonas".parse::<CapitalizedName>().is_ok());

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
    fn test_deref() {
        libcnb_newtype!(CapitalizedName, CapitalizedNameError, r"^[A-Z][a-z]*$");

        fn foo(name: &str) {
            assert_eq!(name, "Johanna");
        }

        let name = "Johanna".parse::<CapitalizedName>().unwrap();
        foo(&name);
    }
}
