use crate::newtypes::libcnb_newtype;

libcnb_newtype!(
    buildpack,
    /// Construct a [`BuildpackId`] value at compile time.
    ///
    /// Passing a string that is not a valid `BuildpackId` value will yield a compilation error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::buildpack_id;
    /// use libcnb_data::buildpack::BuildpackId;
    ///
    /// let buildpack_id: BuildpackId = buildpack_id!("heroku/java");
    /// ```
    buildpack_id,
    /// The ID of a buildpack.
    ///
    /// It MUST only contain numbers, letters, and the characters `.`, `/`, and `-`.
    /// It also MUST NOT be `config` or `app`.
    ///
    /// Use the [`buildpack_id`](crate::buildpack_id) macro to construct a `BuildpackId` from a
    /// literal string. To parse a dynamic string into a `BuildpackId`, use
    /// [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::buildpack::BuildpackId;
    /// use libcnb_data::buildpack_id;
    ///
    /// let from_literal = buildpack_id!("heroku/jvm");
    ///
    /// let input = "heroku/jvm";
    /// let from_dynamic: BuildpackId = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "app";
    /// let invalid: Result<BuildpackId, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    BuildpackId,
    BuildpackIdError,
    r"^(?!app$|config$)[[:alnum:]./-]+$"
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn buildpack_id_does_not_allow_app() {
        let result = BuildpackId::from_str("app");
        assert!(result.is_err());
    }

    #[test]
    fn buildpack_id_does_not_allow_config() {
        let result = BuildpackId::from_str("config");
        assert!(result.is_err());
    }
}
