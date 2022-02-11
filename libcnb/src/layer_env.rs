//! Type-safe, in-memory, layer environment variables.

use crate::Env;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

/// Represents environment variable modifications of a Cloud Native Buildpack layer.
///
/// Cloud Native Buildpacks can add a special directory to their layer directories to modify the
/// environment of subsequent buildpacks, the running container, or specific processes at launch.
/// The rules for these modifications are described in the [relevant section of the specification](https://github.com/buildpacks/spec/blob/main/buildpack.md#provided-by-the-buildpacks).
///
/// This type decouples this information from the file system, providing a type-safe in-memory
/// representation of the environment delta that is specified in the `env/*` directories of a layer.
/// Using this type, libcnb can provide declarative APIs that enable buildpack authors to easily
/// test their layer environment variable logic since they no longer write them to disk manually.
///
/// One use-case is environment variables that are modified by a layer that are required by the
/// same buildpack in later stages of the build process. For example, a buildpack might install a
/// build tool (e.g. Apache Maven) in one layer and add the main binary to `PATH` via the `env`
/// directory of that layer. The same buildpack then wants to execute Maven to download dependencies
/// to a different layer. By using `LayerEnv`, the buildpack can encode these changes in a
/// type and, in addition to passing it to libcnb which will persist it to disk, pass it to the
/// logic that uses the build tool to download dependencies. The download process does not need to
/// know the layer name or any of the logic for constructing `PATH`.
///
/// # Applying the delta
///`LayerEnv` is not a static set of environment variables, but a delta. Layers can modify existing
/// variables by appending, prepending or setting variables only if they were not already defined. If you only need a
/// static set of environment variables, see [`Env`].
///
/// To apply a `LayerEnv` delta to a given `Env`, use [`LayerEnv::apply`] like so:
///```
/// use libcnb::layer_env::{LayerEnv, Scope, ModificationBehavior};
/// use libcnb::Env;
///
/// let mut layer_env = LayerEnv::new();
/// layer_env.insert(Scope::All, ModificationBehavior::Append, "VAR", "bar");
/// layer_env.insert(Scope::All, ModificationBehavior::Default, "VAR2", "default");
///
/// let mut env = Env::new();
/// env.insert("VAR", "foo");
/// env.insert("VAR2", "previous-value");
///
/// let modified_env = layer_env.apply(Scope::Build, &env);
/// assert_eq!(modified_env.get("VAR").unwrap(), "foobar");
/// assert_eq!(modified_env.get("VAR2").unwrap(), "previous-value");
/// ```
///
/// # Implicit Entries
/// Some directories in a layer directory are implicitly added to the layer environment if they
/// exist. The prime example for this behaviour is the `bin` directory. If it exists, its path will
/// be automatically appended to `PATH` using the operating systems path delimiter as the delimiter.
///
/// A full list of these special directories can be found in the
/// [Cloud Native Buildpack specification](https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-paths).
///
/// libcnb supports these, including all precedence and lifecycle rules, when a `LayerEnv` is read
/// from disk:
///```
/// use libcnb::layer_env::{LayerEnv, Scope};
/// use tempfile::tempdir;
/// use libcnb::Env;
/// use std::fs;
///
/// // Create a bogus layer directory
/// let temp_dir = tempdir().unwrap();
/// let layer_dir = temp_dir.path();
/// fs::create_dir_all(layer_dir.join("bin")).unwrap();
/// fs::create_dir_all(layer_dir.join("include")).unwrap();
///
/// let layer_env = LayerEnv::read_from_layer_dir(&layer_dir).unwrap();
/// let env = layer_env.apply_to_empty(Scope::Launch);
///
/// assert_eq!(env.get("PATH").unwrap(), layer_dir.join("bin"));
/// assert_eq!(env.get("CPATH"), None); // None, because CPATH is only added during build
/// ```
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub struct LayerEnv {
    all: LayerEnvDelta,
    build: LayerEnvDelta,
    launch: LayerEnvDelta,
    process: HashMap<String, LayerEnvDelta>,

    // Entries for the standard layer paths as described in the CNB spec:
    // https://github.com/buildpacks/spec/blob/a9f64de9c78022aa7a5091077a765f932d7afe42/buildpack.md#layer-paths
    // These cannot be set by the user itself and are only populated when a `LayerEnv` is read from
    // disk by this library.
    layer_paths_build: LayerEnvDelta,
    layer_paths_launch: LayerEnvDelta,
}

impl LayerEnv {
    /// Creates an empty `LayerEnv` that does not modify any environment variables.
    ///
    /// Entries can be added with the [`LayerEnv::insert`] function.
    ///
    /// # Example:
    /// ```
    /// use libcnb::layer_env::{LayerEnv, Scope};
    /// use libcnb::Env;
    ///
    /// let layer_env = LayerEnv::new();
    /// let mut env = Env::new();
    ///
    /// let modified_env = layer_env.apply(Scope::Build, &env);
    /// assert_eq!(env, modified_env);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies this [`LayerEnv`] to the given [`Env`] for the given [`Scope`].
    ///
    /// For applying to an empty [`Env`], see [`apply_to_empty`](Self::apply_to_empty).
    ///
    /// # Example:
    ///```
    /// use libcnb::layer_env::{LayerEnv, Scope, ModificationBehavior};
    /// use libcnb::Env;
    ///
    /// let mut layer_env = LayerEnv::new();
    /// layer_env.insert(Scope::All, ModificationBehavior::Append, "VAR", "bar");
    /// layer_env.insert(Scope::All, ModificationBehavior::Default, "VAR2", "default");
    ///
    /// let mut env = Env::new();
    /// env.insert("VAR", "foo");
    /// env.insert("VAR2", "previous-value");
    ///
    /// let modified_env = layer_env.apply(Scope::Build, &env);
    /// assert_eq!(modified_env.get("VAR").unwrap(), "foobar");
    /// assert_eq!(modified_env.get("VAR2").unwrap(), "previous-value");
    /// ```
    #[must_use]
    pub fn apply(&self, scope: Scope, env: &Env) -> Env {
        let deltas = match scope {
            Scope::All => vec![&self.all],
            Scope::Build => vec![&self.all, &self.build, &self.layer_paths_build],
            Scope::Launch => vec![&self.all, &self.launch, &self.layer_paths_launch],
            Scope::Process(process) => {
                let mut process_deltas = vec![&self.all];
                if let Some(process_specific_delta) = self.process.get(&process) {
                    process_deltas.push(process_specific_delta);
                }

                process_deltas
            }
        };

        deltas
            .iter()
            .fold(env.clone(), |env, delta| delta.apply(&env))
    }

    /// Applies this [`LayerEnv`] to an empty [`Env`] for the given [`Scope`].
    ///
    /// For applying to an existing [`Env`], see [`apply`](Self::apply).
    #[must_use]
    pub fn apply_to_empty(&self, scope: Scope) -> Env {
        self.apply(scope, &Env::new())
    }

    /// Insert a new entry into this `LayerEnv`.
    ///
    /// Should there already be an entry for the same scope, modification behavior and
    /// name, it will be updated with the new given value.
    ///
    /// # Example:
    /// ```
    /// use libcnb::layer_env::{LayerEnv, Scope, ModificationBehavior};
    /// use libcnb::Env;
    ///
    /// let mut layer_env = LayerEnv::new();
    /// layer_env.insert(Scope::All, ModificationBehavior::Default, "VAR", "hello");
    /// // "foo" will be overridden by "bar" here:
    /// layer_env.insert(Scope::All, ModificationBehavior::Append, "VAR2", "foo");
    /// layer_env.insert(Scope::All, ModificationBehavior::Append, "VAR2", "bar");
    ///
    /// let env = layer_env.apply_to_empty(Scope::Build);
    ///
    /// assert_eq!(env.get("VAR").unwrap(), "hello");
    /// assert_eq!(env.get("VAR2").unwrap(), "bar");
    /// ```
    ///
    /// See [`LayerEnv::chainable_insert`] that allows easy creation of inline `LayerEnv` values
    /// without intermediate variables.
    pub fn insert(
        &mut self,
        scope: Scope,
        modification_behavior: ModificationBehavior,
        name: impl Into<OsString>,
        value: impl Into<OsString>,
    ) {
        let target_delta = match scope {
            Scope::All => &mut self.all,
            Scope::Build => &mut self.build,
            Scope::Launch => &mut self.launch,
            Scope::Process(process_type_name) => match self.process.entry(process_type_name) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(LayerEnvDelta::new()),
            },
        };

        target_delta.insert(modification_behavior, name, value);
    }

    /// Chainable version of [`LayerEnv::insert`].
    ///
    /// Allows easy creation of inline `LayerEnv` values without intermediate variables. See
    /// [`LayerEnv::insert`] docs for details.
    ///
    /// # Example:
    /// ```
    /// use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
    /// use libcnb::Env;
    ///
    /// fn something_that_needs_layer_env(layer_env: LayerEnv) {
    ///     let env = layer_env.apply_to_empty(Scope::Build);
    ///     assert_eq!(env.get("VAR").unwrap(), "hello");
    ///     assert_eq!(env.get("VAR2").unwrap(), "bar");
    /// }
    ///
    /// something_that_needs_layer_env(
    ///     LayerEnv::new()
    ///         .chainable_insert(
    ///             Scope::All,
    ///             ModificationBehavior::Default,
    ///             "VAR",
    ///             "hello",
    ///         )
    ///         .chainable_insert(
    ///             Scope::All,
    ///             ModificationBehavior::Append,
    ///             "VAR2",
    ///             "bar",
    ///         ),
    /// );
    /// ```
    #[must_use]
    pub fn chainable_insert(
        mut self,
        scope: Scope,
        modification_behavior: ModificationBehavior,
        name: impl Into<OsString>,
        value: impl Into<OsString>,
    ) -> Self {
        self.insert(scope, modification_behavior, name, value);
        self
    }

    /// Constructs a `LayerEnv` based on the given layer directory.
    ///
    /// Follows the rules described in the Cloud Native Buildpacks specification and adds implicit
    /// entries for special directories (such as `bin`) should they exist.
    ///
    /// **NOTE**: Buildpack authors should **never directly use this** in their buildpack code and
    /// rely on libcnb to pass `LayerEnv` values to minimize side effects in buildpack code.
    ///
    /// # Example:
    ///```
    /// use libcnb::layer_env::{LayerEnv, Scope};
    /// use tempfile::tempdir;
    /// use libcnb::Env;
    /// use std::fs;
    ///
    /// // Create a bogus layer directory
    /// let temp_dir = tempdir().unwrap();
    /// let layer_dir = temp_dir.path();
    /// fs::create_dir_all(layer_dir.join("bin")).unwrap();
    ///
    /// let layer_env_dir = layer_dir.join("env");
    /// fs::create_dir_all(&layer_env_dir).unwrap();
    /// fs::write(layer_env_dir.join("ZERO_WING.default"), "ALL_YOUR_BASE_ARE_BELONG_TO_US").unwrap();
    ///
    /// let layer_env = LayerEnv::read_from_layer_dir(&layer_dir).unwrap();
    /// let env = layer_env.apply_to_empty(Scope::Launch);
    ///
    /// assert_eq!(env.get("PATH").unwrap(), layer_dir.join("bin"));
    /// assert_eq!(env.get("ZERO_WING").unwrap(), "ALL_YOUR_BASE_ARE_BELONG_TO_US");
    /// ```
    pub fn read_from_layer_dir(layer_dir: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let mut result_layer_env = Self::new();

        let bin_path = layer_dir.as_ref().join("bin");
        let lib_path = layer_dir.as_ref().join("lib");
        let include_path = layer_dir.as_ref().join("include");
        let pkgconfig_path = layer_dir.as_ref().join("pkgconfig");

        let layer_path_specs = vec![
            ("PATH", Scope::Build, &bin_path),
            ("LIBRARY_PATH", Scope::Build, &lib_path),
            ("LD_LIBRARY_PATH", Scope::Build, &lib_path),
            ("CPATH", Scope::Build, &include_path),
            ("PKG_CONFIG_PATH", Scope::Build, &pkgconfig_path),
            ("PATH", Scope::Launch, &bin_path),
            ("LD_LIBRARY_PATH", Scope::Launch, &lib_path),
        ];

        for (name, scope, path) in layer_path_specs {
            if path.is_dir() {
                let target_delta = match scope {
                    Scope::Build => &mut result_layer_env.layer_paths_build,
                    Scope::Launch => &mut result_layer_env.layer_paths_launch,
                    _ => unreachable!("Unexpected Scope in read_from_layer_dir implementation. This is a libcnb implementation error!"),
                };

                target_delta.insert(ModificationBehavior::Prepend, &name, &path);
                target_delta.insert(ModificationBehavior::Delimiter, &name, PATH_LIST_SEPARATOR);
            }
        }

        let env_path = layer_dir.as_ref().join("env");
        if env_path.is_dir() {
            result_layer_env.all = LayerEnvDelta::read_from_env_dir(env_path)?;
        }

        let env_build_path = layer_dir.as_ref().join("env.build");
        if env_build_path.is_dir() {
            result_layer_env.build = LayerEnvDelta::read_from_env_dir(env_build_path)?;
        }

        let env_launch_path = layer_dir.as_ref().join("env.launch");
        if env_launch_path.is_dir() {
            result_layer_env.launch = LayerEnvDelta::read_from_env_dir(env_launch_path)?;
        }

        Ok(result_layer_env)
    }

    /// Writes this `LayerEnv` to the given layer directory.
    ///
    /// **WARNING:** Existing files that configure the layer environment will be deleted!
    ///
    /// **NOTE**: Buildpack authors should **never directly use this** in their buildpack code and
    /// rely on libcnb's declarative APIs to write `LayerEnv` values to disk to minimize side
    /// effects in buildpack code.
    ///
    /// Example:
    /// ```
    /// use libcnb::layer_env::{LayerEnv, Scope, ModificationBehavior};
    /// use tempfile::tempdir;
    /// use std::fs;
    ///
    /// let mut layer_env = LayerEnv::new();
    /// layer_env.insert(Scope::Build, ModificationBehavior::Default, "FOO", "bar");
    /// layer_env.insert(Scope::All, ModificationBehavior::Append, "PATH", "some-path");
    ///
    /// let mut temp_dir = tempdir().unwrap();
    /// layer_env.write_to_layer_dir(&temp_dir).unwrap();
    ///
    /// assert_eq!(fs::read_to_string(temp_dir.path().join("env.build").join("FOO.default")).unwrap(), "bar");
    /// assert_eq!(fs::read_to_string(temp_dir.path().join("env").join("PATH.append")).unwrap(), "some-path");
    /// ```
    pub fn write_to_layer_dir(&self, layer_dir: impl AsRef<Path>) -> std::io::Result<()> {
        self.all.write_to_env_dir(layer_dir.as_ref().join("env"))?;

        self.build
            .write_to_env_dir(layer_dir.as_ref().join("env.build"))?;

        let launch_env_dir = layer_dir.as_ref().join("env.launch");

        self.launch.write_to_env_dir(&launch_env_dir)?;

        for (process_name, delta) in &self.process {
            delta.write_to_env_dir(launch_env_dir.join(process_name))?;
        }

        Ok(())
    }
}

/// Environment variable modification behavior.
/// ([CNB spec: Environment Variable Modification Rules](https://github.com/buildpacks/spec/blob/main/buildpack.md#environment-variable-modification-rules))
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ModificationBehavior {
    Append,
    Default,
    Delimiter,
    Override,
    Prepend,
}

impl Ord for ModificationBehavior {
    fn cmp(&self, other: &Self) -> Ordering {
        // Explicit mapping used over macro based approach to avoid tying source order of elements
        // to ordering logic.
        fn index(value: &ModificationBehavior) -> i32 {
            match value {
                ModificationBehavior::Append => 0,
                ModificationBehavior::Default => 1,
                ModificationBehavior::Delimiter => 2,
                ModificationBehavior::Override => 3,
                ModificationBehavior::Prepend => 4,
            }
        }

        index(self).cmp(&index(other))
    }
}

impl PartialOrd for ModificationBehavior {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The scope of an environment variable modification.
#[derive(Eq, PartialEq, Debug)]
pub enum Scope {
    All,
    Build,
    Launch,
    Process(String),
}

#[derive(Eq, PartialEq, Debug, Default, Clone)]
struct LayerEnvDelta {
    entries: BTreeMap<(ModificationBehavior, OsString), OsString>,
}

impl LayerEnvDelta {
    fn new() -> Self {
        Self::default()
    }

    fn apply(&self, env: &Env) -> Env {
        let mut result_env = env.clone();

        for ((modification_behavior, name), value) in &self.entries {
            match modification_behavior {
                ModificationBehavior::Override => {
                    result_env.insert(&name, &value);
                }
                ModificationBehavior::Default => {
                    if !result_env.contains_key(&name) {
                        result_env.insert(&name, &value);
                    }
                }
                ModificationBehavior::Append => {
                    let mut previous_value = result_env.get(&name).unwrap_or_default();

                    if previous_value.len() > 0 {
                        previous_value.push(self.delimiter_for(&name));
                    }

                    previous_value.push(&value);

                    result_env.insert(&name, previous_value);
                }
                ModificationBehavior::Prepend => {
                    let previous_value = result_env.get(&name).unwrap_or_default();

                    let mut new_value = OsString::new();
                    new_value.push(&value);

                    if !previous_value.is_empty() {
                        new_value.push(self.delimiter_for(&name));
                        new_value.push(previous_value);
                    }

                    result_env.insert(&name, new_value);
                }
                ModificationBehavior::Delimiter => (),
            };
        }

        result_env
    }

    fn delimiter_for(&self, key: impl Into<OsString>) -> OsString {
        self.entries
            .get(&(ModificationBehavior::Delimiter, key.into()))
            .cloned()
            .unwrap_or_default()
    }

    fn read_from_env_dir(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let mut layer_env = Self::new();

        for dir_entry in fs::read_dir(path.as_ref())? {
            // The CNB spec explicitly states:
            //
            // > File contents MUST NOT be evaluated by a shell or otherwise modified before
            // > inclusion in environment variable values.
            // > https://github.com/buildpacks/spec/blob/a9f64de9c78022aa7a5091077a765f932d7afe42/buildpack.md#provided-by-the-buildpacks
            //
            // This should include parsing the contents with an assumed charset and later emitting
            // the raw bytes of that encoding as it might change the actual data. Since this is not
            // explicitly written in the spec, we read through the the reference implementation and
            // determined that it also treats the file contents as raw bytes.
            // See: https://github.com/buildpacks/lifecycle/blob/a7428a55c2a14d8a37e84285b95dc63192e3264e/env/env.go#L73-L106
            let path = dir_entry?.path();

            #[cfg(target_family = "unix")]
            let file_contents = {
                use std::os::unix::ffi::OsStringExt;
                OsString::from_vec(fs::read(&path)?)
            };
            #[cfg(not(target_family = "unix"))]
            let file_contents = OsString::from(&fs::read_to_string(&path)?);

            // Rely on the Rust standard library for splitting stem and extension. Since paths
            // are not necessarily UTF-8 encoded, this is not as trivial as it might look like.
            // Think twice before changing this.
            let file_name_stem = path.file_stem();
            let file_name_extension = path.extension();

            if let Some(file_name_stem) = file_name_stem {
                let modification_behavior = match file_name_extension {
                    None => Some(ModificationBehavior::Override),
                    Some(file_name_extension) => match file_name_extension.to_str() {
                        Some("append") => Some(ModificationBehavior::Append),
                        Some("default") => Some(ModificationBehavior::Default),
                        Some("delim") => Some(ModificationBehavior::Delimiter),
                        Some("override") => Some(ModificationBehavior::Override),
                        Some("prepend") => Some(ModificationBehavior::Prepend),
                        // Note: This IS NOT the case where we have no extension. This handles
                        // the case of an unknown or non-UTF-8 extension.
                        Some(_) | None => None,
                    },
                };

                if let Some(modification_behavior) = modification_behavior {
                    layer_env.insert(
                        modification_behavior,
                        file_name_stem.to_os_string(),
                        file_contents,
                    );
                }
            }
        }

        Ok(layer_env)
    }

    fn write_to_env_dir(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        if path.as_ref().exists() {
            // This is a possible race condition if the path is deleted between the check and
            // removal by this code. We accept this for now to keep it simple.
            fs::remove_dir_all(path.as_ref())?;
        }

        fs::create_dir_all(path.as_ref())?;

        for ((modification_behavior, name), value) in &self.entries {
            let file_extension = match modification_behavior {
                ModificationBehavior::Append => ".append",
                ModificationBehavior::Default => ".default",
                ModificationBehavior::Delimiter => ".delim",
                ModificationBehavior::Override => ".override",
                ModificationBehavior::Prepend => ".prepend",
            };

            let mut file_name = name.clone();
            file_name.push(file_extension);

            let file_path = path.as_ref().join(file_name);

            #[cfg(target_family = "unix")]
            {
                use std::os::unix::ffi::OsStrExt;
                fs::write(file_path, &value.as_bytes())?;
            }

            #[cfg(not(target_family = "unix"))]
            fs::write(file_path, &value.to_string_lossy().as_bytes())?;
        }

        Ok(())
    }

    fn insert(
        &mut self,
        modification_behavior: ModificationBehavior,
        name: impl Into<OsString>,
        value: impl Into<OsString>,
    ) -> &Self {
        self.entries
            .insert((modification_behavior, name.into()), value.into());

        self
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::fs;

    use tempfile::tempdir;

    use crate::layer_env::{Env, LayerEnv, ModificationBehavior, Scope};

    use super::LayerEnvDelta;

    /// Direct port of a test from the reference lifecycle implementation:
    /// See: <https://github.com/buildpacks/lifecycle/blob/a7428a55c2a14d8a37e84285b95dc63192e3264e/env/env_test.go#L105-L154>
    #[test]
    fn reference_impl_env_files_have_a_suffix_it_performs_the_matching_action() {
        let temp_dir = tempdir().unwrap();

        let mut files = HashMap::new();
        files.insert("VAR_APPEND.append", "value-append");
        files.insert("VAR_APPEND_NEW.append", "value-append");
        files.insert("VAR_APPEND_DELIM.append", "value-append-delim");
        files.insert("VAR_APPEND_DELIM_NEW.append", "value-append-delim");
        files.insert("VAR_APPEND_DELIM.delim", "[]");
        files.insert("VAR_APPEND_DELIM_NEW.delim", "[]");

        files.insert("VAR_PREPEND.prepend", "value-prepend");
        files.insert("VAR_PREPEND_NEW.prepend", "value-prepend");
        files.insert("VAR_PREPEND_DELIM.prepend", "value-prepend-delim");
        files.insert("VAR_PREPEND_DELIM_NEW.prepend", "value-prepend-delim");
        files.insert("VAR_PREPEND_DELIM.delim", "[]");
        files.insert("VAR_PREPEND_DELIM_NEW.delim", "[]");

        files.insert("VAR_DEFAULT.default", "value-default");
        files.insert("VAR_DEFAULT_NEW.default", "value-default");

        files.insert("VAR_OVERRIDE.override", "value-override");
        files.insert("VAR_OVERRIDE_NEW.override", "value-override");

        files.insert("VAR_IGNORE.ignore", "value-ignore");

        for (file_name, file_contents) in files {
            fs::write(temp_dir.path().join(file_name), file_contents).unwrap();
        }

        let mut original_env = Env::new();
        original_env.insert("VAR_APPEND", "value-append-orig");
        original_env.insert("VAR_APPEND_DELIM", "value-append-delim-orig");
        original_env.insert("VAR_PREPEND", "value-prepend-orig");
        original_env.insert("VAR_PREPEND_DELIM", "value-prepend-delim-orig");
        original_env.insert("VAR_DEFAULT", "value-default-orig");
        original_env.insert("VAR_OVERRIDE", "value-override-orig");

        let layer_env_delta = LayerEnvDelta::read_from_env_dir(temp_dir.path()).unwrap();
        let modified_env = layer_env_delta.apply(&original_env);

        assert_eq!(
            vec![
                ("VAR_APPEND", "value-append-origvalue-append"),
                (
                    "VAR_APPEND_DELIM",
                    "value-append-delim-orig[]value-append-delim"
                ),
                ("VAR_APPEND_DELIM_NEW", "value-append-delim"),
                ("VAR_APPEND_NEW", "value-append"),
                ("VAR_DEFAULT", "value-default-orig"),
                ("VAR_DEFAULT_NEW", "value-default"),
                ("VAR_OVERRIDE", "value-override"),
                ("VAR_OVERRIDE_NEW", "value-override"),
                ("VAR_PREPEND", "value-prependvalue-prepend-orig"),
                (
                    "VAR_PREPEND_DELIM",
                    "value-prepend-delim[]value-prepend-delim-orig"
                ),
                ("VAR_PREPEND_DELIM_NEW", "value-prepend-delim"),
                ("VAR_PREPEND_NEW", "value-prepend"),
            ],
            environment_as_sorted_vector(&modified_env)
        );
    }

    /// Direct port of a test from the reference lifecycle implementation:
    /// See: <https://github.com/buildpacks/lifecycle/blob/a7428a55c2a14d8a37e84285b95dc63192e3264e/env/env_test.go#L188-L210>
    #[test]
    fn reference_impl_env_files_have_no_suffix_default_action_is_override() {
        let temp_dir = tempdir().unwrap();

        let mut files = HashMap::new();
        files.insert("VAR_NORMAL", "value-normal");
        files.insert("VAR_NORMAL_NEW", "value-normal");
        files.insert("VAR_NORMAL_DELIM", "value-normal-delim");
        files.insert("VAR_NORMAL_DELIM_NEW", "value-normal-delim");
        files.insert("VAR_NORMAL_DELIM.delim", "[]");
        files.insert("VAR_NORMAL_DELIM_NEW.delim", "[]");

        for (file_name, file_contents) in files {
            fs::write(temp_dir.path().join(file_name), file_contents).unwrap();
        }

        let mut original_env = Env::new();
        original_env.insert("VAR_NORMAL", "value-normal-orig");
        original_env.insert("VAR_NORMAL_DELIM", "value-normal-delim-orig");

        let layer_env_delta = LayerEnvDelta::read_from_env_dir(temp_dir.path()).unwrap();
        let modified_env = layer_env_delta.apply(&original_env);

        assert_eq!(
            vec![
                ("VAR_NORMAL", "value-normal"),
                ("VAR_NORMAL_DELIM", "value-normal-delim"),
                ("VAR_NORMAL_DELIM_NEW", "value-normal-delim"),
                ("VAR_NORMAL_NEW", "value-normal"),
            ],
            environment_as_sorted_vector(&modified_env)
        );
    }

    /// Direct port of a test from the reference lifecycle implementation:
    /// See: <https://github.com/buildpacks/lifecycle/blob/a7428a55c2a14d8a37e84285b95dc63192e3264e/env/env_test.go#L55-L80>
    #[test]
    fn reference_impl_add_root_dir_should_append_posix_directories() {
        let temp_dir = tempdir().unwrap();
        fs::create_dir_all(temp_dir.path().join("bin")).unwrap();
        fs::create_dir_all(temp_dir.path().join("lib")).unwrap();

        let mut original_env = Env::new();
        original_env.insert("PATH", "some");
        original_env.insert("LD_LIBRARY_PATH", "some-ld");
        original_env.insert("LIBRARY_PATH", "some-library");

        let layer_env = LayerEnv::read_from_layer_dir(temp_dir.path()).unwrap();
        let modified_env = layer_env.apply(Scope::Build, &original_env);

        assert_eq!(
            vec![
                (
                    "LD_LIBRARY_PATH",
                    format!("{}:some-ld", temp_dir.path().join("lib").to_str().unwrap()).as_str()
                ),
                (
                    "LIBRARY_PATH",
                    format!(
                        "{}:some-library",
                        temp_dir.path().join("lib").to_str().unwrap()
                    )
                    .as_str()
                ),
                (
                    "PATH",
                    format!("{}:some", temp_dir.path().join("bin").to_str().unwrap()).as_str()
                )
            ],
            environment_as_sorted_vector(&modified_env)
        );
    }

    #[test]
    fn layer_env_delta_fs_read_write() {
        let mut original_delta = LayerEnvDelta::new();
        original_delta.insert(ModificationBehavior::Append, "APPEND_TO_ME", "NEW_VALUE");
        original_delta.insert(
            ModificationBehavior::Default,
            "SET_THE_DEFAULT",
            "DEFAULT_VALUE",
        );
        original_delta.insert(ModificationBehavior::Delimiter, "APPEND_TO_ME", ";");
        original_delta.insert(
            ModificationBehavior::Override,
            "OVERRIDE_THIS",
            "OVERRIDE_VALUE",
        );
        original_delta.insert(
            ModificationBehavior::Prepend,
            "PREPEND_THIS",
            "PREPEND_VALUE",
        );

        let temp_dir = tempdir().unwrap();

        original_delta.write_to_env_dir(&temp_dir.path()).unwrap();
        let disk_delta = LayerEnvDelta::read_from_env_dir(&temp_dir.path()).unwrap();

        assert_eq!(original_delta, disk_delta);
    }

    #[test]
    fn layer_env_insert() {
        let mut layer_env = LayerEnv::new();
        layer_env.insert(
            Scope::Build,
            ModificationBehavior::Append,
            "MAVEN_OPTS",
            "-Dskip.tests=true",
        );

        layer_env.insert(
            Scope::All,
            ModificationBehavior::Override,
            "JAVA_TOOL_OPTIONS",
            "-Xmx1G",
        );

        layer_env.insert(
            Scope::Build,
            ModificationBehavior::Override,
            "JAVA_TOOL_OPTIONS",
            "-Xmx2G",
        );

        layer_env.insert(
            Scope::Launch,
            ModificationBehavior::Append,
            "JAVA_TOOL_OPTIONS",
            "-XX:+UseSerialGC",
        );

        let result_env = layer_env.apply_to_empty(Scope::Build);
        assert_eq!(
            vec![
                ("JAVA_TOOL_OPTIONS", "-Xmx2G"),
                ("MAVEN_OPTS", "-Dskip.tests=true")
            ],
            environment_as_sorted_vector(&result_env)
        );
    }

    #[test]
    fn modification_behavior_order() {
        let tests = vec![
            (
                ModificationBehavior::Append,
                ModificationBehavior::Default,
                Ordering::Less,
            ),
            (
                ModificationBehavior::Append,
                ModificationBehavior::Override,
                Ordering::Less,
            ),
            (
                ModificationBehavior::Prepend,
                ModificationBehavior::Append,
                Ordering::Greater,
            ),
            (
                ModificationBehavior::Default,
                ModificationBehavior::Delimiter,
                Ordering::Less,
            ),
            (
                ModificationBehavior::Default,
                ModificationBehavior::Default,
                Ordering::Equal,
            ),
        ];

        for (a, b, expected) in tests {
            assert_eq!(expected, a.cmp(&b));
        }
    }

    #[test]
    fn layer_env_delta_eq() {
        let mut delta_1 = LayerEnvDelta::new();
        delta_1.insert(ModificationBehavior::Default, "a", "avalue");
        delta_1.insert(ModificationBehavior::Default, "b", "bvalue");
        delta_1.insert(ModificationBehavior::Override, "c", "cvalue");

        let mut delta_2 = LayerEnvDelta::new();
        delta_2.insert(ModificationBehavior::Default, "b", "bvalue");
        delta_2.insert(ModificationBehavior::Override, "c", "cvalue");
        delta_2.insert(ModificationBehavior::Default, "a", "avalue");

        assert_eq!(delta_1, delta_2);
    }

    #[test]
    fn read_from_layer_dir_layer_paths_launch() {
        let temp_dir = tempdir().unwrap();
        let layer_dir = temp_dir.path();

        // https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-paths
        fs::create_dir_all(layer_dir.join("bin")).unwrap();
        fs::create_dir_all(layer_dir.join("lib")).unwrap();
        fs::create_dir_all(layer_dir.join("include")).unwrap();
        fs::create_dir_all(layer_dir.join("pkgconfig")).unwrap();

        let layer_env = LayerEnv::read_from_layer_dir(&layer_dir).unwrap();

        let env = layer_env.apply_to_empty(Scope::Launch);
        assert_eq!(env.get("PATH").unwrap(), layer_dir.join("bin"));
        assert_eq!(env.get("LD_LIBRARY_PATH").unwrap(), layer_dir.join("lib"));
        assert_eq!(env.get("LIBRARY_PATH"), None);
        assert_eq!(env.get("CPATH"), None);
        assert_eq!(env.get("PKG_CONFIG_PATH"), None);
    }

    #[test]
    fn read_from_layer_dir_layer_paths_build() {
        let temp_dir = tempdir().unwrap();
        let layer_dir = temp_dir.path();

        // https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-paths
        fs::create_dir_all(layer_dir.join("bin")).unwrap();
        fs::create_dir_all(layer_dir.join("lib")).unwrap();
        fs::create_dir_all(layer_dir.join("include")).unwrap();
        fs::create_dir_all(layer_dir.join("pkgconfig")).unwrap();

        let layer_env = LayerEnv::read_from_layer_dir(&layer_dir).unwrap();

        let env = layer_env.apply_to_empty(Scope::Build);
        assert_eq!(env.get("PATH").unwrap(), layer_dir.join("bin"));
        assert_eq!(env.get("LD_LIBRARY_PATH").unwrap(), layer_dir.join("lib"));
        assert_eq!(env.get("LIBRARY_PATH").unwrap(), layer_dir.join("lib"));
        assert_eq!(env.get("CPATH").unwrap(), layer_dir.join("include"));
        assert_eq!(
            env.get("PKG_CONFIG_PATH").unwrap(),
            layer_dir.join("pkgconfig")
        );
    }

    fn environment_as_sorted_vector(environment: &Env) -> Vec<(&str, &str)> {
        let mut result: Vec<(&str, &str)> = environment
            .iter()
            .map(|(k, v)| (k.to_str().unwrap(), v.to_str().unwrap()))
            .collect();

        result.sort_by_key(|kv| kv.0);
        result
    }
}

#[cfg(target_family = "unix")]
const PATH_LIST_SEPARATOR: &str = ":";

#[cfg(target_family = "windows")]
const PATH_LIST_SEPARATOR: &str = ";";
