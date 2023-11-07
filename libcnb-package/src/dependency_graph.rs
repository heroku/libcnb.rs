use petgraph::visit::DfsPostOrder;
use petgraph::Graph;
use std::error::Error;

/// A node of a dependency graph.
///
/// See: [`create_dependency_graph`]
pub trait DependencyNode<T, E>
where
    T: PartialEq,
{
    fn id(&self) -> T;

    /// The dependencies of a node
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the dependencies can't be accessed
    fn dependencies(&self) -> Result<Vec<T>, E>;
}

/// Create a [`Graph`] from [`DependencyNode`]s.
///
/// # Errors
///
/// Will return an `Err` if the graph contains references to missing dependencies or the
/// dependencies of a [`DependencyNode`] couldn't be gathered.
pub fn create_dependency_graph<T, I, E>(
    nodes: Vec<T>,
) -> Result<Graph<T, ()>, CreateDependencyGraphError<I, E>>
where
    T: DependencyNode<I, E>,
    I: PartialEq,
    E: Error,
{
    let mut graph = Graph::new();

    for node in nodes {
        graph.add_node(node);
    }

    for idx in graph.node_indices() {
        let node = &graph[idx];

        let dependencies = node
            .dependencies()
            .map_err(CreateDependencyGraphError::GetNodeDependenciesError)?;

        for dependency in dependencies {
            let dependency_idx = graph
                .node_indices()
                .find(|idx| graph[*idx].id() == dependency)
                .ok_or(CreateDependencyGraphError::MissingDependency(dependency))?;

            graph.add_edge(idx, dependency_idx, ());
        }
    }

    Ok(graph)
}

/// An error from [`create_dependency_graph`]
#[derive(thiserror::Error, Debug)]
pub enum CreateDependencyGraphError<I, E: Error> {
    #[error("Error while determining dependencies of a node: {0}")]
    GetNodeDependenciesError(#[source] E),
    #[error("Node references unknown dependency {0}")]
    MissingDependency(I),
}

/// Collects all the [`DependencyNode`] values found while traversing the given dependency graph
/// using one or more `root_nodes` values as starting points for the traversal. The returned list
/// will contain the given `root_nodes` values as well as all their dependencies in topological order.
///
/// # Errors
///
/// Will return an `Err` if the graph contains references to missing dependencies.
pub fn get_dependencies<'a, T, I, E>(
    graph: &'a Graph<T, ()>,
    root_nodes: &[&T],
) -> Result<Vec<&'a T>, GetDependenciesError<I>>
where
    T: DependencyNode<I, E>,
    I: PartialEq,
{
    let mut order: Vec<&T> = Vec::new();
    let mut dfs = DfsPostOrder::empty(&graph);
    for root_node in root_nodes {
        let idx = graph
            .node_indices()
            .find(|idx| graph[*idx].id() == root_node.id())
            .ok_or(GetDependenciesError::UnknownRootNode(root_node.id()))?;

        dfs.move_to(idx);

        while let Some(visited) = dfs.next(&graph) {
            order.push(&graph[visited]);
        }
    }
    Ok(order)
}

/// An error from [`get_dependencies`]
#[derive(thiserror::Error, Debug)]
pub enum GetDependenciesError<I> {
    #[error("Root node {0} is not in the dependency graph")]
    UnknownRootNode(I),
}

#[cfg(test)]
mod tests {
    use crate::dependency_graph::{create_dependency_graph, get_dependencies, DependencyNode};
    use std::convert::Infallible;

    impl DependencyNode<String, Infallible> for (&str, Vec<&str>) {
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
    fn test_get_dependencies_one_level_deep() {
        let a = ("a", Vec::new());
        let b = ("b", Vec::new());
        let c = ("c", vec!["a", "b"]);

        let graph = create_dependency_graph(vec![a.clone(), b.clone(), c.clone()]).unwrap();

        assert_eq!(get_dependencies(&graph, &[&a]).unwrap(), &[&a]);

        assert_eq!(get_dependencies(&graph, &[&b]).unwrap(), &[&b]);

        assert_eq!(get_dependencies(&graph, &[&c]).unwrap(), &[&a, &b, &c]);

        assert_eq!(
            &get_dependencies(&graph, &[&b, &c, &a]).unwrap(),
            &[&b, &a, &c]
        );
    }

    #[test]
    fn test_get_dependencies_two_levels_deep() {
        let a = ("a", Vec::new());
        let b = ("b", vec!["a"]);
        let c = ("c", vec!["b"]);

        let graph = create_dependency_graph(vec![a.clone(), b.clone(), c.clone()]).unwrap();

        assert_eq!(get_dependencies(&graph, &[&a]).unwrap(), &[&a]);

        assert_eq!(get_dependencies(&graph, &[&b]).unwrap(), &[&a, &b]);

        assert_eq!(get_dependencies(&graph, &[&c]).unwrap(), &[&a, &b, &c]);

        assert_eq!(
            &get_dependencies(&graph, &[&b, &c, &a]).unwrap(),
            &[&a, &b, &c]
        );
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn test_get_dependencies_with_overlap() {
        let a = ("a", Vec::new());
        let b = ("b", Vec::new());
        let c = ("c", Vec::new());
        let d = ("d", vec!["a", "b"]);
        let e = ("e", vec!["b", "c"]);

        let graph =
            create_dependency_graph(vec![a.clone(), b.clone(), c.clone(), d.clone(), e.clone()])
                .unwrap();

        assert_eq!(
            get_dependencies(&graph, &[&d, &e, &a]).unwrap(),
            &[&a, &b, &d, &c, &e]
        );

        assert_eq!(
            get_dependencies(&graph, &[&e, &d, &a]).unwrap(),
            &[&b, &c, &e, &a, &d]
        );
    }
}
