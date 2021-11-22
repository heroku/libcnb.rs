use crate::newtypes::libcnb_newtype;

libcnb_newtype!(
    layer,
    /// Construct a [`LayerName`] value at compile time.
    ///
    /// Passing a string that is not a valid `LayerName` value will yield a compilation error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::layer_name;
    /// use libcnb_data::layer::LayerName;
    ///
    /// let layer_name: LayerName = layer_name!("foobar");
    /// ```
    layer_name,
    /// The name of a layer.
    ///
    /// It can contain all characters supported by the filesystem, but MUST NOT be either `build`,
    /// `launch` or `store`.
    ///
    /// Use the [`layer_name`](crate::layer_name) macro to construct a `LayerName` from a literal string. To
    /// parse a dynamic string into a `LayerName`, use [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::layer::LayerName;
    /// use libcnb_data::layer_name;
    ///
    /// let from_literal = layer_name!("foobar");
    ///
    /// let input = "foobar";
    /// let from_dynamic: LayerName = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "build";
    /// let invalid: Result<LayerName, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    LayerName,
    LayerNameError,
    r"^(?!build|launch|store).*$"
);
