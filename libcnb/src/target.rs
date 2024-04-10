pub struct Target {
    /// The name of the target operating system.
    ///
    /// The value should conform to [Go's `$GOOS`](https://golang.org/doc/install/source#environment), for example
    /// `linux` or `windows`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's [`os` property](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).
    pub os: String,
    /// The name of the target CPU architecture.
    ///
    /// The value should conform to [Go's $GOARCH](https://golang.org/doc/install/source#environment), for example
    /// `amd64` or `arm64`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's [`architecture` property](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).
    /// ``
    pub arch: String,
    /// The variant of the specified CPU architecture.
    ///
    /// The value should conform to [OCI image spec platform variants](https://github.com/opencontainers/image-spec/blob/main/image-index.md#platform-variants), for example
    /// `v7` or `v8`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's [`variant` property](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).
    pub arch_variant: Option<String>,
    /// The name of the operating system distribution. Should be empty for Windows.
    ///
    /// For example: `ubuntu` or `alpine`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's `io.buildpacks.base.distro.name` label.
    pub distro_name: Option<String>,
    /// The version of the operating system distribution.
    ///
    /// For example: `18.02` or `3.19`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's `io.buildpacks.base.distro.version` label.
    pub distro_version: Option<String>,
}
