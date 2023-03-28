#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod build;
pub mod config;
pub mod cross_compile;

use crate::build::BuildpackBinaries;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId, BuildpackIdError};
use libcnb_data::buildpackage::{Buildpackage, BuildpackageDependency};
use petgraph::graph::NodeIndex;
use petgraph::visit::DfsPostOrder;
use petgraph::Graph;
use std::collections::HashMap;
use std::fs;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};
use toml::Table;

/// The profile to use when invoking Cargo.
///
/// <https://doc.rust-lang.org/cargo/reference/profiles.html>
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CargoProfile {
    /// Provides faster compilation times at the expense of runtime performance and binary size.
    Dev,
    /// Produces assets with optimised runtime performance and binary size, at the expense of compilation time.
    Release,
}

/// A parsed buildpack descriptor and it's path.
#[derive(Debug, Clone)]
pub struct BuildpackData<BM> {
    pub buildpack_descriptor_path: PathBuf,
    pub buildpack_descriptor: BuildpackDescriptor<BM>,
}

/// A parsed buildpackage descriptor and it's path.
#[derive(Debug, Clone)]
pub struct BuildpackageData {
    pub buildpackage_descriptor_path: PathBuf,
    pub buildpackage_descriptor: Buildpackage,
}

/// A convenient type alias to use with [`BuildpackPackage`] or [`BuildpackData`] when you don't required a specialized metadata representation.
pub type GenericMetadata = Option<Table>;

/// A folder that can be packaged into a [Cloud Native Buildpack](https://buildpacks.io/)
#[derive(Debug, Clone)]
pub struct BuildpackPackage<T = GenericMetadata> {
    pub path: PathBuf,
    pub buildpack_data: BuildpackData<T>,
    pub buildpackage_data: Option<BuildpackageData>,
}

impl BuildpackPackage {
    #[must_use]
    pub fn buildpack_id(&self) -> &BuildpackId {
        &self.buildpack_data.buildpack_descriptor.buildpack().id
    }
}

/// A dependency graph of [`BuildpackPackage`]s
pub struct BuildpackPackageGraph {
    graph: Graph<BuildpackPackage, ()>,
}

impl BuildpackPackageGraph {
    #[must_use]
    pub fn packages(&self) -> Vec<&BuildpackPackage> {
        self.graph
            .node_indices()
            .map(|idx| &self.graph[idx])
            .collect::<Vec<_>>()
    }
}

/// Buildpack dependency type
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BuildpackDependency {
    External(BuildpackageDependency),
    Local(BuildpackId, BuildpackageDependency),
}

impl BuildpackDependency {
    #[must_use]
    pub fn get_local_buildpack_id(&self) -> Option<BuildpackId> {
        match self {
            BuildpackDependency::External(_) => None,
            BuildpackDependency::Local(id, _) => Some(id.clone()),
        }
    }
}

/// Reads buildpack data from the given project path.
///
/// # Errors
///
/// Will return `Err` if the buildpack data could not be read successfully.
pub fn read_buildpack_data(
    project_path: impl AsRef<Path>,
) -> Result<BuildpackData<GenericMetadata>, ReadBuildpackDataError> {
    let dir = project_path.as_ref();
    let buildpack_descriptor_path = dir.join("buildpack.toml");
    fs::read_to_string(&buildpack_descriptor_path)
        .map_err(|e| ReadBuildpackDataError::ReadingBuildpack {
            path: buildpack_descriptor_path.clone(),
            source: e,
        })
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(|e| ReadBuildpackDataError::ParsingBuildpack {
                path: buildpack_descriptor_path.clone(),
                source: e,
            })
        })
        .map(|buildpack_descriptor| BuildpackData {
            buildpack_descriptor_path,
            buildpack_descriptor,
        })
}

/// An error from [`read_buildpack_data`]
#[derive(Debug)]
pub enum ReadBuildpackDataError {
    ReadingBuildpack {
        path: PathBuf,
        source: std::io::Error,
    },
    ParsingBuildpack {
        path: PathBuf,
        source: toml::de::Error,
    },
}

/// Reads buildpackage data from the given project path.
///
/// # Errors
///
/// Will return `Err` if the buildpackage data could not be read successfully.
pub fn read_buildpackage_data(
    project_path: impl AsRef<Path>,
) -> Result<Option<BuildpackageData>, ReadBuildpackageDataError> {
    let buildpackage_descriptor_path = project_path.as_ref().join("package.toml");

    if !buildpackage_descriptor_path.exists() {
        return Ok(None);
    }

    fs::read_to_string(&buildpackage_descriptor_path)
        .map_err(|e| ReadBuildpackageDataError::ReadingBuildpackage {
            path: buildpackage_descriptor_path.clone(),
            source: e,
        })
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(|e| {
                ReadBuildpackageDataError::ParsingBuildpackage {
                    path: buildpackage_descriptor_path.clone(),
                    source: e,
                }
            })
        })
        .map(|buildpackage_descriptor| {
            Some(BuildpackageData {
                buildpackage_descriptor_path,
                buildpackage_descriptor,
            })
        })
}

/// An error from [`read_buildpackage_data`]
#[derive(Debug)]
pub enum ReadBuildpackageDataError {
    ReadingBuildpackage {
        path: PathBuf,
        source: std::io::Error,
    },
    ParsingBuildpackage {
        path: PathBuf,
        source: toml::de::Error,
    },
}

/// Reads both the buildpack and buildpackage data from a given project path.
///  
/// # Errors
///
/// Will return an `Err` if either the buildpack or buildpackage data could not be read.
pub fn read_buildpack_package<P: Into<PathBuf>>(
    project_path: P,
) -> Result<BuildpackPackage, ReadBuildpackPackageError> {
    let path = project_path.into();
    let buildpack_data =
        read_buildpack_data(&path).map_err(ReadBuildpackPackageError::ReadBuildpackDataError)?;
    let buildpackage_data = read_buildpackage_data(&path)
        .map_err(ReadBuildpackPackageError::ReadBuildpackageDataError)?;
    Ok(BuildpackPackage {
        path,
        buildpack_data,
        buildpackage_data,
    })
}

/// An error from [`read_buildpack_package`]
#[derive(Debug)]
pub enum ReadBuildpackPackageError {
    ReadBuildpackDataError(ReadBuildpackDataError),
    ReadBuildpackageDataError(ReadBuildpackageDataError),
}

/// Create a [`BuildpackPackageGraph`] from a list of [`BuildpackPackage`] values.
///
/// # Errors
///
/// Will return an `Err` if the constructed dependency graph is missing any local [`BuildpackPackage`] dependencies.
pub fn create_buildpack_package_graph(
    buildpack_packages: Vec<BuildpackPackage>,
) -> Result<BuildpackPackageGraph, CreateBuildpackPackageGraphError> {
    let mut graph = Graph::new();

    for buildpack_package in buildpack_packages {
        graph.add_node(buildpack_package);
    }

    for idx in graph.node_indices() {
        let buildpack_package = &graph[idx];
        let dependencies = buildpack_package
            .buildpackage_data
            .as_ref()
            .map(|value| &value.buildpackage_descriptor)
            .map_or(Ok(vec![]), get_local_buildpackage_dependencies)
            .map_err(CreateBuildpackPackageGraphError::BuildpackIdError)?;
        for dependency in dependencies {
            let dependency_idx = lookup_buildpack_package_node_index(&graph, &dependency).ok_or(
                CreateBuildpackPackageGraphError::BuildpackageLookup(dependency),
            )?;
            graph.add_edge(idx, dependency_idx, ());
        }
    }

    Ok(BuildpackPackageGraph { graph })
}

/// An error from [`create_buildpack_package_graph`]
#[derive(Debug)]
pub enum CreateBuildpackPackageGraphError {
    BuildpackIdError(BuildpackIdError),
    BuildpackageLookup(BuildpackId),
}

/// Collects all the [`BuildpackPackage`] values found while traversing the given `buildpack_packages` graph
/// using one or more `root_packages` values as starting points for the traversal. The returned list
/// will contain the given `root_packages` values as well as all their dependencies in topological order.
///
/// # Errors
///
/// An `Err` will be returned if any [`BuildpackPackage`] located contains a reference to a [`BuildpackPackage`]
/// that is not in the `buildpack_packages` graph.
pub fn get_buildpack_package_dependencies<'a>(
    buildpack_packages: &'a BuildpackPackageGraph,
    root_packages: &[&BuildpackPackage],
) -> Result<Vec<&'a BuildpackPackage>, GetBuildpackPackageDependenciesError> {
    let graph = &buildpack_packages.graph;
    let mut order: Vec<&BuildpackPackage> = vec![];
    let mut dfs = DfsPostOrder::empty(&graph);
    for root in root_packages {
        let idx = lookup_buildpack_package_node_index(graph, root.buildpack_id()).ok_or(
            GetBuildpackPackageDependenciesError::BuildpackPackageLookup(
                root.buildpack_id().clone(),
            ),
        )?;
        dfs.move_to(idx);
        while let Some(visited) = dfs.next(&graph) {
            order.push(&graph[visited]);
        }
    }
    Ok(order)
}

/// An error from [`get_buildpack_package_dependencies`]
#[derive(Debug)]
pub enum GetBuildpackPackageDependenciesError {
    BuildpackPackageLookup(BuildpackId),
}

fn lookup_buildpack_package_node_index(
    graph: &Graph<BuildpackPackage, ()>,
    buildpack_id: &BuildpackId,
) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|idx| graph[*idx].buildpack_id() == buildpack_id)
}

/// Creates a buildpack directory and copies all buildpack assets to it.
///
/// Assembly of the directory follows the constraints set by the libcnb framework. For example,
/// the buildpack binary is only copied once and symlinks are used to refer to it when the CNB
/// spec requires different file(name)s.
///
/// This function will not validate if the buildpack descriptor at the given path is valid and will
/// use it as-is.
///
/// # Errors
///
/// Will return `Err` if the buildpack directory could not be assembled.
pub fn assemble_buildpack_directory(
    destination_path: impl AsRef<Path>,
    buildpack_descriptor_path: impl AsRef<Path>,
    buildpack_binaries: &BuildpackBinaries,
) -> std::io::Result<()> {
    fs::create_dir_all(destination_path.as_ref())?;

    fs::copy(
        buildpack_descriptor_path.as_ref(),
        destination_path.as_ref().join("buildpack.toml"),
    )?;

    let bin_path = destination_path.as_ref().join("bin");
    fs::create_dir_all(&bin_path)?;

    fs::copy(
        &buildpack_binaries.buildpack_target_binary_path,
        bin_path.join("build"),
    )?;

    create_file_symlink("build", bin_path.join("detect"))?;

    if !buildpack_binaries.additional_target_binary_paths.is_empty() {
        let additional_binaries_dir = destination_path
            .as_ref()
            .join(".libcnb-cargo")
            .join("additional-bin");

        fs::create_dir_all(&additional_binaries_dir)?;

        for (binary_target_name, binary_path) in &buildpack_binaries.additional_target_binary_paths
        {
            fs::copy(
                binary_path,
                additional_binaries_dir.join(binary_target_name),
            )?;
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original.as_ref(), link.as_ref())
}

#[cfg(target_family = "windows")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(original.as_ref(), link.as_ref())
}

/// Construct a good default filename for a buildpack directory.
///
/// This function ensures the resulting name is valid and does not contain problematic characters
/// such as `/`.
#[must_use]
pub fn default_buildpack_directory_name(buildpack_id: &BuildpackId) -> String {
    buildpack_id.replace('/', "_")
}

fn get_buildpack_dependencies(
    buildpackage: &Buildpackage,
) -> Result<Vec<BuildpackDependency>, BuildpackIdError> {
    buildpackage
        .dependencies
        .iter()
        .map(|dependency| {
            buildpack_id_from_libcnb_dependency(dependency).map(|buildpack_id| {
                buildpack_id.map_or_else(
                    || BuildpackDependency::External(dependency.clone()),
                    |value| BuildpackDependency::Local(value, dependency.clone()),
                )
            })
        })
        .collect::<Result<_, _>>()
}

fn buildpack_id_from_libcnb_dependency(
    dependency: &BuildpackageDependency,
) -> Result<Option<BuildpackId>, BuildpackIdError> {
    Some(&dependency.uri)
        .filter(|uri| {
            uri.scheme()
                .map_or(false, |scheme| scheme.as_str() == "libcnb")
        })
        .map(|uri| uri.path().to_string().parse())
        .transpose()
}

/// Reads the dependency URIs from the given `buildpackage` and returns any local libcnb project
/// references which should have the format `libcnb:{buildpack_id}`.
///
/// # Errors
///
/// Will return an `Err` if any of the local dependencies use an invalid [`BuildpackId`].
pub fn get_local_buildpackage_dependencies(
    buildpackage: &Buildpackage,
) -> Result<Vec<BuildpackId>, BuildpackIdError> {
    get_buildpack_dependencies(buildpackage).map(|dependencies| {
        dependencies
            .iter()
            .filter_map(BuildpackDependency::get_local_buildpack_id)
            .collect::<Vec<_>>()
    })
}

/// Creates a new [`Buildpackage`] value by replacing each local libcnb dependency with the
/// file path where the compiled dependency is located.
///
/// This assumes that each libcnb dependency has already been compiled and the given
/// `buildpack_ids_to_target_dir` contains the correct mappings of path locations for each
/// [`BuildpackId`].
///
/// # Errors
///
/// Will return an `Err` if:
/// * the given `buildpackage` contains a local dependency with an invalid [`BuildpackId`]
/// * there is no entry found in `buildpack_ids_to_target_dir` for a local dependency's [`BuildpackId`]
/// * the target path for a local dependency is an invalid URI
pub fn rewrite_buildpackage_local_dependencies<S: BuildHasher>(
    buildpackage: &Buildpackage,
    buildpack_ids_to_target_dir: &HashMap<&BuildpackId, PathBuf, S>,
) -> Result<Buildpackage, RewriteBuildpackageLocalDependenciesError> {
    let local_dependency_to_target_dir = |target_dir: &PathBuf| {
        BuildpackageDependency::try_from(target_dir.clone()).map_err(|_| {
            RewriteBuildpackageLocalDependenciesError::InvalidDependency(target_dir.clone())
        })
    };

    get_buildpack_dependencies(buildpackage)
        .map_err(RewriteBuildpackageLocalDependenciesError::GetBuildpackDependenciesError)
        .and_then(|dependencies| {
            dependencies
                .into_iter()
                .map(|dependency| match dependency {
                    BuildpackDependency::External(buildpackage_dependency) => {
                        Ok(buildpackage_dependency)
                    }
                    BuildpackDependency::Local(buildpack_id, _) => buildpack_ids_to_target_dir
                        .get(&buildpack_id)
                        .ok_or(
                            RewriteBuildpackageLocalDependenciesError::TargetDirectoryLookup(
                                buildpack_id,
                            ),
                        )
                        .and_then(local_dependency_to_target_dir),
                })
                .collect()
        })
        .map(|dependencies| Buildpackage {
            dependencies,
            buildpack: buildpackage.buildpack.clone(),
            platform: buildpackage.platform.clone(),
        })
}

/// An error for [`rewrite_buildpackage_local_dependencies`]
#[derive(Debug)]
pub enum RewriteBuildpackageLocalDependenciesError {
    TargetDirectoryLookup(BuildpackId),
    InvalidDependency(PathBuf),
    GetBuildpackDependenciesError(BuildpackIdError),
}

/// Creates a new [`Buildpackage`] value by replacing each relative URI with it's absolute path using
/// the given `source_path`.
///
/// # Errors
///
/// Will return an `Err` if:
/// * the given `buildpackage` contains a local dependency with an invalid [`BuildpackId`]
/// * the constructed absolute path is an invalid URI
pub fn rewrite_buildpackage_relative_path_dependencies_to_absolute(
    buildpackage: &Buildpackage,
    source_dir: &Path,
) -> Result<Buildpackage, RewriteBuildpackageRelativePathDependenciesToAbsoluteError> {
    let relative_dependency_to_absolute =
        |source_dir: &Path, buildpackage_dependency: BuildpackageDependency| {
            let absolute_path = source_dir.join(buildpackage_dependency.uri.path().to_string());
            BuildpackageDependency::try_from(absolute_path.clone()).map_err(|_| {
                RewriteBuildpackageRelativePathDependenciesToAbsoluteError::InvalidDependency(
                    absolute_path,
                )
            })
        };

    get_buildpack_dependencies(buildpackage)
        .map_err(RewriteBuildpackageRelativePathDependenciesToAbsoluteError::GetBuildpackDependenciesError)
        .and_then(|dependencies| {
            dependencies
                .into_iter()
                .map(|dependency| match dependency {
                    BuildpackDependency::External(buildpackage_dependency) => {
                        if buildpackage_dependency.uri.is_relative_path_reference() {
                            relative_dependency_to_absolute(source_dir, buildpackage_dependency)
                        } else {
                            Ok(buildpackage_dependency)
                        }
                    }
                    BuildpackDependency::Local(_, buildpackage_dependency) => {
                        Ok(buildpackage_dependency)
                    }
                })
                .collect()
        })
        .map(|dependencies| Buildpackage {
            dependencies,
            buildpack: buildpackage.buildpack.clone(),
            platform: buildpackage.platform.clone(),
        })
}

/// An error for [`rewrite_buildpackage_relative_path_dependencies_to_absolute`]
#[derive(Debug)]
pub enum RewriteBuildpackageRelativePathDependenciesToAbsoluteError {
    InvalidDependency(PathBuf),
    GetBuildpackDependenciesError(BuildpackIdError),
}

/// Recursively walks the file system from the given `start_dir` to locate any folders containing a
/// `buildpack.toml` file.
///
/// # Errors
///
/// Will return an `Err` if any I/O errors happen while walking the file system.
pub fn find_buildpack_dirs(
    start_dir: &Path,
    options: &FindBuildpackDirsOptions,
) -> Result<Vec<PathBuf>, FindBuildpackDirsError> {
    fn find_buildpack_dirs_recursive(
        path: &Path,
        options: &FindBuildpackDirsOptions,
        accumulator: &mut Vec<PathBuf>,
    ) -> Result<(), FindBuildpackDirsError> {
        if options.ignore.contains(&path.to_path_buf()) {
            return Ok(());
        }

        let metadata = path
            .metadata()
            .map_err(|e| FindBuildpackDirsError::IO(path.to_path_buf(), e))?;

        if metadata.is_dir() {
            let entries = fs::read_dir(path)
                .map_err(|e| FindBuildpackDirsError::IO(path.to_path_buf(), e))?;

            for entry in entries {
                let entry = entry.map_err(|e| FindBuildpackDirsError::IO(path.to_path_buf(), e))?;

                let metadata = entry
                    .metadata()
                    .map_err(|e| FindBuildpackDirsError::IO(entry.path(), e))?;

                if metadata.is_dir() {
                    find_buildpack_dirs_recursive(&entry.path(), options, accumulator)?;
                } else if let Some(file_name) = entry.path().file_name() {
                    if file_name.to_string_lossy() == "buildpack.toml" {
                        accumulator.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(())
    }

    let mut buildpack_dirs: Vec<PathBuf> = vec![];
    find_buildpack_dirs_recursive(start_dir, options, &mut buildpack_dirs)?;
    Ok(buildpack_dirs)
}

/// Options for configuring [`find_buildpack_dirs`]
#[derive(Debug, Default)]
pub struct FindBuildpackDirsOptions {
    pub ignore: Vec<PathBuf>,
}

/// An error for [`find_buildpack_dirs`]
#[derive(Debug)]
pub enum FindBuildpackDirsError {
    IO(PathBuf, std::io::Error),
}

/// Provides a standard path to use for storing a compiled buildpack's artifacts.
#[must_use]
pub fn get_buildpack_target_dir(
    buildpack_id: &BuildpackId,
    target_dir: &Path,
    is_release: bool,
) -> PathBuf {
    target_dir
        .join("buildpack")
        .join(if is_release { "release" } else { "debug" })
        .join(default_buildpack_directory_name(buildpack_id))
}

#[cfg(test)]
mod tests {
    use crate::{
        create_buildpack_package_graph, get_buildpack_package_dependencies,
        get_buildpack_target_dir, get_local_buildpackage_dependencies,
        rewrite_buildpackage_local_dependencies,
        rewrite_buildpackage_relative_path_dependencies_to_absolute, BuildpackData,
        BuildpackPackage, BuildpackageData, GenericMetadata,
    };
    use libcnb_data::buildpack::{
        BuildpackDescriptor, BuildpackId, MetaBuildpackDescriptor, SingleBuildpackDescriptor,
    };
    use libcnb_data::buildpack_id;
    use libcnb_data::buildpackage::{
        Buildpackage, BuildpackageBuildpack, BuildpackageDependency, Platform,
    };
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_rewrite_buildpackage_relative_path_dependencies() {
        let buildpackage = create_buildpackage();
        let source_dir = PathBuf::from("/test/source/path");
        let new_buildpackage =
            rewrite_buildpackage_relative_path_dependencies_to_absolute(&buildpackage, &source_dir)
                .unwrap();
        assert_eq!(
            new_buildpackage.dependencies[1].uri.to_string(),
            "/test/source/path/../relative/path"
        );
    }

    #[test]
    fn test_rewrite_buildpackage_local_dependencies() {
        let buildpackage = create_buildpackage();
        let buildpack_id = buildpack_id!("buildpack-id");
        let buildpack_ids_to_target_dir = HashMap::from([(
            &buildpack_id,
            PathBuf::from("/path/to/target/buildpacks/buildpack-id"),
        )]);
        let new_buildpackage =
            rewrite_buildpackage_local_dependencies(&buildpackage, &buildpack_ids_to_target_dir)
                .unwrap();
        assert_eq!(
            new_buildpackage.dependencies[0].uri.to_string(),
            "/path/to/target/buildpacks/buildpack-id"
        );
    }

    #[test]
    fn test_get_local_buildpackage_dependencies() {
        let buildpackage = create_buildpackage();
        assert_eq!(
            get_local_buildpackage_dependencies(&buildpackage).unwrap(),
            vec![buildpack_id!("buildpack-id")]
        );
    }

    #[test]
    fn test_get_buildpack_target_dir() {
        let buildpack_id = buildpack_id!("some-org/with-buildpack");
        let target_dir = PathBuf::from("/target");
        assert_eq!(
            get_buildpack_target_dir(&buildpack_id, &target_dir, false),
            PathBuf::from("/target/buildpack/debug/some-org_with-buildpack")
        );
        assert_eq!(
            get_buildpack_target_dir(&buildpack_id, &target_dir, true),
            PathBuf::from("/target/buildpack/release/some-org_with-buildpack")
        );
    }

    #[test]
    fn test_get_buildpack_package_dependencies_one_level_deep() {
        let a = create_buildpack_package(&buildpack_id!("a"));
        let b = create_buildpack_package(&buildpack_id!("b"));
        let c = create_meta_buildpack_package(
            &buildpack_id!("c"),
            vec![buildpack_id!("a"), buildpack_id!("b")],
        );

        let buildpack_packages =
            create_buildpack_package_graph(vec![a.clone(), b.clone(), c.clone()]).unwrap();

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[&a]).unwrap()),
            to_ids(&[&a])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[&b]).unwrap()),
            to_ids(&[&b])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[&c]).unwrap()),
            to_ids(&[&a, &b, &c])
        );

        assert_eq!(
            to_ids(
                &get_buildpack_package_dependencies(&buildpack_packages, &[&b, &c, &a]).unwrap()
            ),
            to_ids(&[&b, &a, &c])
        );
    }

    #[test]
    fn test_get_buildpack_package_dependencies_two_levels_deep() {
        let a = create_buildpack_package(&buildpack_id!("a"));
        let b = create_meta_buildpack_package(&buildpack_id!("b"), vec![buildpack_id!("a")]);
        let c = create_meta_buildpack_package(&buildpack_id!("c"), vec![buildpack_id!("b")]);

        let buildpack_packages =
            create_buildpack_package_graph(vec![a.clone(), b.clone(), c.clone()]).unwrap();

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[&a]).unwrap()),
            to_ids(&[&a])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[&b]).unwrap()),
            to_ids(&[&a, &b])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[&c]).unwrap()),
            to_ids(&[&a, &b, &c])
        );

        assert_eq!(
            to_ids(
                &get_buildpack_package_dependencies(&buildpack_packages, &[&b, &c, &a]).unwrap()
            ),
            to_ids(&[&a, &b, &c])
        );
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn test_get_buildpack_package_dependencies_with_overlap() {
        let a = create_buildpack_package(&buildpack_id!("a"));
        let b = create_buildpack_package(&buildpack_id!("b"));
        let c = create_buildpack_package(&buildpack_id!("c"));
        let d = create_meta_buildpack_package(
            &buildpack_id!("d"),
            vec![buildpack_id!("a"), buildpack_id!("b")],
        );
        let e = create_meta_buildpack_package(
            &buildpack_id!("e"),
            vec![buildpack_id!("b"), buildpack_id!("c")],
        );

        let buildpack_packages = create_buildpack_package_graph(vec![
            a.clone(),
            b.clone(),
            c.clone(),
            d.clone(),
            e.clone(),
        ])
        .unwrap();

        assert_eq!(
            to_ids(
                &get_buildpack_package_dependencies(&buildpack_packages, &[&d, &e, &a]).unwrap()
            ),
            to_ids(&[&a, &b, &d, &c, &e])
        );

        assert_eq!(
            to_ids(
                &get_buildpack_package_dependencies(&buildpack_packages, &[&e, &d, &a]).unwrap()
            ),
            to_ids(&[&b, &c, &e, &a, &d])
        );
    }

    fn create_buildpackage() -> Buildpackage {
        create_buildpackage_with_dependencies(vec![
            "libcnb:buildpack-id",
            "../relative/path",
            "/absolute/path",
            "docker://docker.io/heroku/procfile-cnb:2.0.0",
        ])
    }

    fn create_buildpackage_with_dependencies<S>(dependencies: Vec<S>) -> Buildpackage
    where
        S: Into<String>,
    {
        Buildpackage {
            buildpack: BuildpackageBuildpack::try_from(".").unwrap(),
            dependencies: dependencies
                .into_iter()
                .map(|v| BuildpackageDependency::try_from(v.into().as_ref()).unwrap())
                .collect(),
            platform: Platform::default(),
        }
    }

    fn create_single_buildpack_data(
        dir: &Path,
        id: &BuildpackId,
    ) -> BuildpackData<GenericMetadata> {
        let toml_str = format!(
            r#" 
                api = "0.8" 
                [buildpack] 
                id = "{id}" 
                version = "0.0.1" 
                
                [[stacks]]
                id = "some-stack"
            "#
        );
        BuildpackData {
            buildpack_descriptor_path: dir.join("buildpack.toml"),
            buildpack_descriptor: BuildpackDescriptor::Single(
                toml::from_str::<SingleBuildpackDescriptor<GenericMetadata>>(&toml_str).unwrap(),
            ),
        }
    }

    fn create_buildpack_package(id: &BuildpackId) -> BuildpackPackage {
        let path = PathBuf::from("/buildpacks/").join(id.to_string());
        BuildpackPackage {
            path: path.clone(),
            buildpack_data: create_single_buildpack_data(&path, id),
            buildpackage_data: None,
        }
    }

    fn create_meta_buildpack_data(
        dir: &Path,
        id: &BuildpackId,
        dependencies: &[BuildpackId],
    ) -> BuildpackData<GenericMetadata> {
        let toml_str = format!(
            r#" 
                api = "0.8" 
                [buildpack] 
                id = "{id}" 
                version = "0.0.1" 
                
                [[order]]
                {}
            "#,
            dependencies
                .iter()
                .map(|v| format!("[[order.group]]\nid = \"{v}\"\nversion = \"0.0.0\"\n"))
                .collect::<Vec<String>>()
                .join("\n")
        );
        BuildpackData {
            buildpack_descriptor_path: dir.join("buildpack.toml"),
            buildpack_descriptor: BuildpackDescriptor::Meta(
                toml::from_str::<MetaBuildpackDescriptor<GenericMetadata>>(&toml_str).unwrap(),
            ),
        }
    }

    fn create_meta_buildpack_package(
        id: &BuildpackId,
        dependencies: Vec<BuildpackId>,
    ) -> BuildpackPackage {
        let path = PathBuf::from("/meta-buildpacks/").join(id.to_string());
        BuildpackPackage {
            path: path.clone(),
            buildpack_data: create_meta_buildpack_data(&path, id, &dependencies),
            buildpackage_data: Some(BuildpackageData {
                buildpackage_descriptor_path: path.join("package.toml"),
                buildpackage_descriptor: create_buildpackage_with_dependencies(
                    dependencies
                        .into_iter()
                        .map(|v| format!("libcnb:{v}"))
                        .collect(),
                ),
            }),
        }
    }

    fn to_ids(buildpackage: &[&BuildpackPackage]) -> Vec<BuildpackId> {
        buildpackage
            .iter()
            .map(|v| v.buildpack_id().clone())
            .collect::<Vec<_>>()
    }
}
