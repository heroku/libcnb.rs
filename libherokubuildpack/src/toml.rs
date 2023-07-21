use std::ops::Deref;

/// Selects a value at the given TOML path from a TOML value.
///
/// Works similarly to an xpath query to select a value inside a complex XML document. This function
/// is useful to select a value from a TOML document without deserializing it into specific type
/// which can sometimes be a complex endeavour.
///
/// Example:
/// ```
/// use libherokubuildpack::toml::toml_select_value;
/// use toml::toml;
///
/// let toml = toml! {
///     [config]
///     [config.net]
///     port = 12345
///     host = "localhost"
/// };
///
/// assert_eq!(
///     toml_select_value(vec!["config", "net", "port"], &toml.into()),
///     Some(&toml::Value::from(12345))
/// );
/// ```
pub fn toml_select_value<S: AsRef<str>, K: Deref<Target = [S]>>(
    keys: K,
    value: &toml::Value,
) -> Option<&toml::Value> {
    if keys.is_empty() {
        Some(value)
    } else {
        match &value {
            toml::Value::Table(table) => keys.split_first().and_then(|(head, tail)| {
                table
                    .get(head.as_ref())
                    .and_then(|next_value| toml_select_value(tail, next_value))
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::toml::toml_select_value;
    use std::collections::HashMap;
    use toml::toml;

    #[test]
    fn test_common_case() {
        let toml = toml! {
            [bogus]
            value = "Will it trip it up?"
            [now]
            this = "is podracing!"
        };

        assert_eq!(
            toml_select_value(vec!["now", "this"], &toml.into()),
            Some(&toml::Value::from("is podracing!"))
        );
    }

    #[test]
    fn test_value_from_dotted_keys() {
        let toml = toml! {
            now.this.is = "podracing"
            [bogus]
            value = "Will it trip it up?"
        };

        assert_eq!(
            toml_select_value(vec!["now", "this", "is"], &toml.into()),
            Some(&toml::Value::from("podracing"))
        );
    }

    #[test]
    fn test_value_from_table() {
        let toml = toml! {
            [bogus]
            value = "Will it trip it up?"

            [now.this]
            is = "podracing"
        };

        assert_eq!(
            toml_select_value(vec!["now", "this", "is"], &toml.into()),
            Some(&toml::Value::from("podracing"))
        );
    }

    #[test]
    fn test_partial_match() {
        let toml = toml! {
            [bogus]
            value = "Will it trip it up?"

            [now.this]
            is = "podracing"
        };

        assert_eq!(
            toml_select_value(vec!["now", "this", "was"], &toml.into()),
            None
        );
    }

    #[test]
    fn test_does_not_modify_value_types() {
        let toml = toml! {
            [translations]
            leet = 1337
        };

        assert_eq!(
            toml_select_value(vec!["translations", "leet"], &toml.into()),
            Some(&toml::Value::from(1337))
        );
    }

    #[test]
    fn test_works_without_keys() {
        let toml = toml! {
            foo = "bar"
        };

        let mut hash_map = HashMap::new();
        hash_map.insert(String::from("foo"), String::from("bar"));

        assert_eq!(
            toml_select_value::<&str, Vec<&str>>(vec![], &toml.into()),
            Some(&toml::Value::from(hash_map))
        );
    }
}
