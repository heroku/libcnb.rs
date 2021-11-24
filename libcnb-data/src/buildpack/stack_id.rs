use crate::newtypes::libcnb_newtype;

libcnb_newtype!(
    buildpack,
    /// Construct a [`StackId`] value at compile time.
    ///
    /// Passing a string that is not a valid `StackId` value will yield a compilation error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::stack_id;
    /// use libcnb_data::buildpack::StackId;
    ///
    /// let stack_id: StackId = stack_id!("heroku-20");
    /// ```
    stack_id,
    /// The ID of a stack.
    ///
    /// It MUST only contain numbers, letters, and the characters `.`, `/`, and `-`.
    ///
    /// Use the [`stack_id`](crate::stack_id) macro to construct a `StackId` from a
    /// literal string. To parse a dynamic string into a `StackId`, use
    /// [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::buildpack::StackId;
    /// use libcnb_data::stack_id;
    ///
    /// let from_literal = stack_id!("heroku-20");
    ///
    /// let input = "heroku-20";
    /// let from_dynamic: StackId = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "_stack_";
    /// let invalid: Result<StackId, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    StackId,
    StackIdError,
    r"^[[:alnum:]./-]+$"
);
