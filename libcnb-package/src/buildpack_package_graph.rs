use petgraph::graph::NodeIndex;
use petgraph::visit::DfsPostOrder;
use petgraph::Graph;

/// A trait to support topological sorting in [`BuildpackPackageGraph`]
pub trait TopoSort<T, E>
where
    T: PartialEq,
{
    /// The id of a node
    fn id(&self) -> T;

    /// The dependencies of a node
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the dependencies can't be accessed
    fn dependencies(&self) -> Result<Vec<T>, E>;
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

        let dependencies = buildpack_package
            .dependencies()
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
    use crate::buildpack_package_graph::{
        create_buildpack_package_graph, get_buildpack_package_dependencies, TopoSort,
    };
    use std::convert::Infallible;

    impl TopoSort<String, Infallible> for (&str, Vec<&str>) {
        fn id(&self) -> String {
            self.0.to_string()
        }

        fn dependencies(&self) -> Result<Vec<String>, Infallible> {
            Ok(self
                .1
                .iter()
                .map(std::string::ToString::to_string)
                .collect())
        }
    }

    #[test]
    fn test_get_buildpack_package_dependencies_one_level_deep() {
        let a = ("a", vec![]);
        let b = ("b", vec![]);
        let c = ("c", vec!["a", "b"]);

        let graph = create_buildpack_package_graph(vec![a.clone(), b.clone(), c.clone()]).unwrap();

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&a]).unwrap(),
            &[&a]
        );

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&b]).unwrap(),
            &[&b]
        );

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&c]).unwrap(),
            &[&a, &b, &c]
        );

        assert_eq!(
            &get_buildpack_package_dependencies(&graph, &[&b, &c, &a]).unwrap(),
            &[&b, &a, &c]
        );
    }

    #[test]
    fn test_get_buildpack_package_dependencies_two_levels_deep() {
        let a = ("a", vec![]);
        let b = ("b", vec!["a"]);
        let c = ("c", vec!["b"]);

        let graph = create_buildpack_package_graph(vec![a.clone(), b.clone(), c.clone()]).unwrap();

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&a]).unwrap(),
            &[&a]
        );

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&b]).unwrap(),
            &[&a, &b]
        );

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&c]).unwrap(),
            &[&a, &b, &c]
        );

        assert_eq!(
            &get_buildpack_package_dependencies(&graph, &[&b, &c, &a]).unwrap(),
            &[&a, &b, &c]
        );
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn test_get_buildpack_package_dependencies_with_overlap() {
        let a = ("a", vec![]);
        let b = ("b", vec![]);
        let c = ("c", vec![]);
        let d = ("d", vec!["a", "b"]);
        let e = ("e", vec!["b", "c"]);

        let graph = create_buildpack_package_graph(vec![
            a.clone(),
            b.clone(),
            c.clone(),
            d.clone(),
            e.clone(),
        ])
        .unwrap();

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&d, &e, &a]).unwrap(),
            &[&a, &b, &d, &c, &e]
        );

        assert_eq!(
            get_buildpack_package_dependencies(&graph, &[&e, &d, &a]).unwrap(),
            &[&b, &c, &e, &a, &d]
        );
    }
}
