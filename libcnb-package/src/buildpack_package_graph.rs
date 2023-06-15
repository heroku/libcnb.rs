use crate::buildpack_dependency::get_local_buildpackage_dependencies;
use crate::buildpack_package::BuildpackPackage;
use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use petgraph::graph::NodeIndex;
use petgraph::visit::DfsPostOrder;
use petgraph::Graph;

pub trait TopoSort<T, E>
where
    T: PartialEq,
{
    fn id(&self) -> T;
    fn deps(&self) -> Result<Vec<T>, E>;
}

/// A dependency graph of [`BuildpackPackage`]s
pub struct BuildpackPackageGraph<T> {
    graph: Graph<T, ()>,
}

impl<T> BuildpackPackageGraph<T> {
    #[must_use]
    pub fn packages(&self) -> Vec<&T> {
        self.graph
            .node_indices()
            .map(|idx| &self.graph[idx])
            .collect::<Vec<_>>()
    }
}

/// Create a [`BuildpackPackageGraph`] from a list of [`BuildpackPackage`] values.
///
/// # Errors
///
/// Will return an `Err` if the constructed dependency graph is missing any local [`BuildpackPackage`] dependencies.
pub fn create_buildpack_package_graph<T, I, E>(
    buildpack_packages: Vec<T>,
) -> Result<BuildpackPackageGraph<T>, CreateBuildpackPackageGraphError<I, E>>
where
    T: TopoSort<I, E>,
    I: PartialEq,
{
    let mut graph = Graph::new();

    for buildpack_package in buildpack_packages {
        graph.add_node(buildpack_package);
    }

    for idx in graph.node_indices() {
        let buildpack_package = &graph[idx];

        let depedencies = buildpack_package
            .deps()
            .map_err(CreateBuildpackPackageGraphError::BuildpackIdError)?;

        for dependency in depedencies {
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
pub enum CreateBuildpackPackageGraphError<I, E> {
    BuildpackIdError(E),
    BuildpackageLookup(I),
}

/// Collects all the [`BuildpackPackage`] values found while traversing the given `buildpack_packages` graph
/// using one or more `root_packages` values as starting points for the traversal. The returned list
/// will contain the given `root_packages` values as well as all their dependencies in topological order.
///
/// # Errors
///
/// An `Err` will be returned if any [`BuildpackPackage`] located contains a reference to a [`BuildpackPackage`]
/// that is not in the `buildpack_packages` graph.
pub fn get_buildpack_package_dependencies<'a, T, I, E>(
    buildpack_packages: &'a BuildpackPackageGraph<T>,
    root_packages: &[&T],
) -> Result<Vec<&'a T>, GetBuildpackPackageDependenciesError<I>>
where
    T: TopoSort<I, E>,
    I: PartialEq,
{
    let graph = &buildpack_packages.graph;
    let mut order: Vec<&T> = vec![];
    let mut dfs = DfsPostOrder::empty(&graph);
    for root in root_packages {
        let idx = lookup_buildpack_package_node_index(graph, &root.id())
            .ok_or(GetBuildpackPackageDependenciesError::BuildpackPackageLookup(root.id()))?;
        dfs.move_to(idx);
        while let Some(visited) = dfs.next(&graph) {
            order.push(&graph[visited]);
        }
    }
    Ok(order)
}

/// An error from [`get_buildpack_package_dependencies`]
#[derive(Debug)]
pub enum GetBuildpackPackageDependenciesError<I> {
    BuildpackPackageLookup(I),
}

fn lookup_buildpack_package_node_index<T, I, E>(graph: &Graph<T, ()>, id: &I) -> Option<NodeIndex>
where
    T: TopoSort<I, E>,
    I: PartialEq,
{
    graph.node_indices().find(|idx| graph[*idx].id() == *id)
}

#[cfg(test)]
mod tests {
    use crate::buildpack_package::BuildpackPackage;
    use crate::buildpack_package_graph::{
        create_buildpack_package_graph, get_buildpack_package_dependencies, BuildpackPackageGraph,
    };
    use crate::{BuildpackData, BuildpackageData, GenericMetadata};
    use libcnb_data::buildpack::{
        BuildpackDescriptor, BuildpackId, MetaBuildpackDescriptor, SingleBuildpackDescriptor,
    };
    use libcnb_data::buildpack_id;
    use libcnb_data::buildpackage::{
        Buildpackage, BuildpackageBuildpackReference, BuildpackageDependency, Platform,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn test_get_buildpack_package_dependencies_one_level_deep() {
        let a = create_buildpack_package(&buildpack_id!("a"));
        let b = create_buildpack_package(&buildpack_id!("b"));
        let c = create_meta_buildpack_package(
            &buildpack_id!("c"),
            vec![buildpack_id!("a"), buildpack_id!("b")],
        );

        let buildpack_packages = create_buildpack_package_graph(vec![a, b, c]).unwrap();

        let a = get_node(&buildpack_packages, "a");
        let b = get_node(&buildpack_packages, "b");
        let c = get_node(&buildpack_packages, "c");

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[a]).unwrap()),
            to_ids(&[a])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[b]).unwrap()),
            to_ids(&[b])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[c]).unwrap()),
            to_ids(&[a, b, c])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[b, c, a]).unwrap()),
            to_ids(&[b, a, c])
        );
    }

    #[test]
    fn test_get_buildpack_package_dependencies_two_levels_deep() {
        let a = create_buildpack_package(&buildpack_id!("a"));
        let b = create_meta_buildpack_package(&buildpack_id!("b"), vec![buildpack_id!("a")]);
        let c = create_meta_buildpack_package(&buildpack_id!("c"), vec![buildpack_id!("b")]);

        let buildpack_packages = create_buildpack_package_graph(vec![a, b, c]).unwrap();

        let a = get_node(&buildpack_packages, "a");
        let b = get_node(&buildpack_packages, "b");
        let c = get_node(&buildpack_packages, "c");

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[a]).unwrap()),
            to_ids(&[a])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[b]).unwrap()),
            to_ids(&[a, b])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[c]).unwrap()),
            to_ids(&[a, b, c])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[b, c, a]).unwrap()),
            to_ids(&[a, b, c])
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

        let buildpack_packages = create_buildpack_package_graph(vec![a, b, c, d, e]).unwrap();

        let a = get_node(&buildpack_packages, "a");
        let b = get_node(&buildpack_packages, "b");
        let c = get_node(&buildpack_packages, "c");
        let d = get_node(&buildpack_packages, "d");
        let e = get_node(&buildpack_packages, "e");

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[d, e, a]).unwrap()),
            to_ids(&[a, b, d, c, e])
        );

        assert_eq!(
            to_ids(&get_buildpack_package_dependencies(&buildpack_packages, &[e, d, a]).unwrap()),
            to_ids(&[b, c, e, a, d])
        );
    }

    fn to_ids(buildpackage: &[&BuildpackPackage]) -> Vec<BuildpackId> {
        buildpackage
            .iter()
            .map(|v| v.buildpack_id().clone())
            .collect::<Vec<_>>()
    }

    fn get_node<'a>(
        buildpack_packages: &'a BuildpackPackageGraph<BuildpackPackage>,
        id: &str,
    ) -> &'a BuildpackPackage {
        let id = id.parse::<BuildpackId>().unwrap();
        let index = buildpack_packages
            .graph
            .node_indices()
            .find(|idx| buildpack_packages.graph[*idx].buildpack_id() == &id)
            .unwrap();
        &buildpack_packages.graph[index]
    }

    fn create_buildpack_package(id: &BuildpackId) -> BuildpackPackage {
        let path = PathBuf::from("/buildpacks/").join(id.to_string());
        BuildpackPackage {
            path: path.clone(),
            buildpack_data: create_single_buildpack_data(&path, id),
            buildpackage_data: None,
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

    fn create_buildpackage_with_dependencies<S>(dependencies: Vec<S>) -> Buildpackage
    where
        S: Into<String>,
    {
        Buildpackage {
            buildpack: BuildpackageBuildpackReference::try_from(".").unwrap(),
            dependencies: dependencies
                .into_iter()
                .map(|v| BuildpackageDependency::try_from(v.into().as_ref()).unwrap())
                .collect(),
            platform: Platform::default(),
        }
    }
}
