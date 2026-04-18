//! Node filtering for partial pipeline execution.

use std::collections::HashSet;
use std::prelude::v1::*;

use serde::Serialize;

use crate::error::PondError;
use crate::graph::build_pipeline_graph;

use super::dyn_steps::StepVec;
use super::traits::{ptr_to_id, DatasetEvent, DatasetRef, StepInfo, RunnableStep};
use super::steps::{PipelineInfo, Steps};

/// Specifies which nodes to include in a filtered pipeline run.
pub enum NodeFilter {
    /// Run only the named nodes.
    Nodes(HashSet<String>),
    /// Run the subgraph between from-nodes and to-nodes.
    /// If `from` is empty, include all ancestors of `to`.
    /// If `to` is empty, include all descendants of `from`.
    FromTo {
        from: HashSet<String>,
        to: HashSet<String>,
    },
}

/// Filter a pipeline's steps, returning a `StepVec` containing only the
/// nodes that match the filter. Pipeline structure is preserved: sub-pipelines
/// whose children partially match are emitted as `DynPipeline` wrappers.
pub fn filter_steps<'a, E>(
    pipe: &'a impl Steps<E>,
    catalog: &impl Serialize,
    params: &impl Serialize,
    filter: &NodeFilter,
) -> Result<StepVec<'a, E>, PondError>
where
    E: From<PondError> + Send + Sync + 'static,
{
    let graph = build_pipeline_graph(pipe, catalog, params);

    // Resolve filter to a set of node names to keep
    let keep_names = resolve_keep_set(&graph, filter)?;

    // Build a set of graph node IDs (ptr-based) for fast lookup during tree walk
    let keep_ids: HashSet<usize> = graph
        .nodes
        .iter()
        .filter(|n| !n.is_pipe && keep_names.contains(n.name))
        .map(|n| n.id)
        .collect();

    // Walk the Steps tree and collect matching items
    let mut result: StepVec<'a, E> = Vec::new();
    pipe.for_each_item(&mut |item| {
        collect_filtered(item, &keep_ids, &mut result);
    });

    Ok(result)
}

/// Resolve a `NodeFilter` into the set of leaf node names to keep.
fn resolve_keep_set(
    graph: &crate::graph::PipelineGraph<'_>,
    filter: &NodeFilter,
) -> Result<HashSet<&'static str>, PondError> {
    let leaf_names: HashSet<&str> = graph
        .nodes
        .iter()
        .filter(|n| !n.is_pipe)
        .map(|n| n.name)
        .collect();

    match filter {
        NodeFilter::Nodes(names) => {
            for name in names {
                if !leaf_names.contains(name.as_str()) {
                    return Err(PondError::NodeNotFound(name.clone()));
                }
            }
            Ok(graph
                .nodes
                .iter()
                .filter(|n| !n.is_pipe && names.contains(n.name))
                .map(|n| n.name)
                .collect())
        }
        NodeFilter::FromTo { from, to } => {
            for name in from.iter().chain(to.iter()) {
                if !leaf_names.contains(name.as_str()) {
                    return Err(PondError::NodeNotFound(name.clone()));
                }
            }
            resolve_from_to(graph, from, to)
        }
    }
}

/// Compute the subgraph between from-nodes and to-nodes using edge traversal.
fn resolve_from_to(
    graph: &crate::graph::PipelineGraph<'_>,
    from: &HashSet<String>,
    to: &HashSet<String>,
) -> Result<HashSet<&'static str>, PondError> {
    let leaves: Vec<usize> = graph
        .node_indices
        .iter()
        .copied()
        .collect();

    // Map node names to graph indices (leaves only)
    let name_to_idx: std::collections::HashMap<&str, usize> = leaves
        .iter()
        .map(|&i| (graph.nodes[i].name, i))
        .collect();

    // Build adjacency lists from edges
    let mut forward: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    let mut backward: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for edge in &graph.edges {
        forward.entry(edge.from_node).or_default().push(edge.to_node);
        backward.entry(edge.to_node).or_default().push(edge.from_node);
    }

    // Forward reachable from `from` nodes (descendants)
    let forward_set = if from.is_empty() {
        // No from constraint — all nodes are forward-reachable
        leaves.iter().copied().collect::<HashSet<usize>>()
    } else {
        let seeds: Vec<usize> = from.iter().map(|n| name_to_idx[n.as_str()]).collect();
        reachable(&seeds, &forward)
    };

    // Backward reachable from `to` nodes (ancestors)
    let backward_set = if to.is_empty() {
        // No to constraint — all nodes are backward-reachable
        leaves.iter().copied().collect::<HashSet<usize>>()
    } else {
        let seeds: Vec<usize> = to.iter().map(|n| name_to_idx[n.as_str()]).collect();
        reachable(&seeds, &backward)
    };

    // Intersect
    let keep_indices: HashSet<usize> = forward_set.intersection(&backward_set).copied().collect();

    Ok(keep_indices
        .iter()
        .map(|&i| graph.nodes[i].name)
        .collect())
}

/// BFS from seed nodes along the given adjacency.
fn reachable(
    seeds: &[usize],
    adj: &std::collections::HashMap<usize, Vec<usize>>,
) -> HashSet<usize> {
    let mut visited = HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    for &s in seeds {
        if visited.insert(s) {
            queue.push_back(s);
        }
    }
    while let Some(node) = queue.pop_front() {
        if let Some(neighbors) = adj.get(&node) {
            for &n in neighbors {
                if visited.insert(n) {
                    queue.push_back(n);
                }
            }
        }
    }
    visited
}

/// Recursively collect filtered steps from a single item.
fn collect_filtered<'a, E>(
    item: &'a dyn RunnableStep<E>,
    keep_ids: &HashSet<usize>,
    out: &mut StepVec<'a, E>,
) where
    E: From<PondError> + Send + Sync + 'static,
{
    if item.is_leaf() {
        let id = ptr_to_id(item.as_pipeline_info());
        if keep_ids.contains(&id) {
            out.push(Box::new(item));
        }
    } else {
        // Pipeline: recurse into children, collect survivors
        let mut children: StepVec<'a, E> = Vec::new();
        item.for_each_child_step(&mut |child| {
            collect_filtered(child, keep_ids, &mut children);
        });
        if !children.is_empty() {
            let mut inputs = Vec::new();
            item.for_each_input(&mut |d| inputs.push(*d));
            let mut outputs = Vec::new();
            item.for_each_output(&mut |d| outputs.push(*d));
            out.push(Box::new(DynPipeline {
                name: item.name(),
                inputs,
                outputs,
                steps: children,
            }));
        }
    }
}

/// A dynamically-constructed pipeline container used by node filtering.
///
/// Mirrors `Pipeline` but stores inputs/outputs as `Vec<DatasetRef>` and
/// children as a `StepVec`, allowing construction from filtered tree walks.
struct DynPipeline<'a, E> {
    name: &'static str,
    inputs: Vec<DatasetRef<'a>>,
    outputs: Vec<DatasetRef<'a>>,
    steps: StepVec<'a, E>,
}

// SAFETY: DynPipeline contains only Send+Sync fields (DatasetRef is Copy,
// StepVec requires Send+Sync on its elements).
unsafe impl<E> Send for DynPipeline<'_, E> {}
unsafe impl<E> Sync for DynPipeline<'_, E> {}

impl<E> StepInfo for DynPipeline<'_, E>
where
    E: Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn type_string(&self) -> &'static str {
        "pipeline"
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepInfo)) {
        self.steps.for_each_info(f);
    }

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for d in &self.inputs {
            f(d);
        }
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for d in &self.outputs {
            f(d);
        }
    }
}

impl<E> RunnableStep<E> for DynPipeline<'_, E>
where
    E: From<PondError> + Send + Sync + 'static,
{
    fn call(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        unreachable!("DynPipeline::call() should not be invoked directly")
    }

    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {
        self.steps.for_each_item(f);
    }

    fn as_pipeline_info(&self) -> &dyn StepInfo {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{MemoryDataset, Param};
    use crate::pipeline::{Node, Pipeline};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Cat {
        a: MemoryDataset<i32>,
        b: MemoryDataset<i32>,
        c: MemoryDataset<i32>,
        d: MemoryDataset<i32>,
    }

    #[derive(Serialize)]
    struct Params {
        x: Param<i32>,
    }

    /// Helper: collect leaf node names from a Steps.
    fn leaf_names<E>(steps: &impl Steps<E>) -> Vec<&'static str> {
        let mut names = Vec::new();
        steps.for_each_item(&mut |item| {
            collect_leaf_names(item, &mut names);
        });
        names
    }

    fn collect_leaf_names<E>(item: &dyn RunnableStep<E>, names: &mut Vec<&'static str>) {
        if item.is_leaf() {
            names.push(item.name());
        } else {
            item.for_each_child_step(&mut |child| {
                collect_leaf_names(child, names);
            });
        }
    }

    #[test]
    fn filter_nodes_flat_pipeline() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },
            Node { name: "n3", func: |v| (v,), input: (&cat.b,), output: (&cat.c,) },
        );

        let filter = NodeFilter::Nodes(["n1", "n3"].iter().map(|s| s.to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        assert_eq!(leaf_names(&filtered), ["n1", "n3"]);
    }

    #[test]
    fn filter_nodes_preserves_pipeline_structure() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },
                    Node { name: "n3", func: |v| (v,), input: (&cat.b,), output: (&cat.c,) },
                ),
                input: (&cat.a,),
                output: (&cat.c,),
            },
        );

        // Keep only n2 — should appear inside a DynPipeline wrapping "inner"
        let filter = NodeFilter::Nodes(["n2"].iter().map(|s| s.to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        assert_eq!(leaf_names(&filtered), ["n2"]);

        // Verify pipeline structure is preserved (one top-level item that is not a leaf)
        let mut top_items = Vec::new();
        filtered.for_each_item(&mut |item| top_items.push((item.name(), item.is_leaf())));
        assert_eq!(top_items, [("inner", false)]);
    }

    #[test]
    fn filter_from_to_subgraph() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        // x -> a (n1) -> b (n2) -> c (n3) -> d (n4)
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },
            Node { name: "n3", func: |v| (v,), input: (&cat.b,), output: (&cat.c,) },
            Node { name: "n4", func: |v| (v,), input: (&cat.c,), output: (&cat.d,) },
        );

        let filter = NodeFilter::FromTo {
            from: ["n2"].iter().map(|s| s.to_string()).collect(),
            to: ["n3"].iter().map(|s| s.to_string()).collect(),
        };
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        assert_eq!(leaf_names(&filtered), ["n2", "n3"]);
    }

    #[test]
    fn filter_from_only() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },
            Node { name: "n3", func: |v| (v,), input: (&cat.b,), output: (&cat.c,) },
        );

        let filter = NodeFilter::FromTo {
            from: ["n2"].iter().map(|s| s.to_string()).collect(),
            to: HashSet::new(),
        };
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        assert_eq!(leaf_names(&filtered), ["n2", "n3"]);
    }

    #[test]
    fn filter_to_only() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },
            Node { name: "n3", func: |v| (v,), input: (&cat.b,), output: (&cat.c,) },
        );

        let filter = NodeFilter::FromTo {
            from: HashSet::new(),
            to: ["n2"].iter().map(|s| s.to_string()).collect(),
        };
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        assert_eq!(leaf_names(&filtered), ["n1", "n2"]);
    }

    #[test]
    fn filter_unknown_node_returns_error() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
        );

        let filter = NodeFilter::Nodes(["nonexistent"].iter().map(|s| s.to_string()).collect());
        let result = filter_steps::<PondError>(&pipe, &cat, &params, &filter);

        assert!(matches!(result, Err(PondError::NodeNotFound(ref s)) if s == "nonexistent"));
    }

    #[test]
    fn filter_skips_empty_pipeline() {
        let cat = Cat {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
            d: MemoryDataset::new(),
        };
        let params = Params { x: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },
                ),
                input: (&cat.a,),
                output: (&cat.b,),
            },
        );

        // Keep only n1 — the inner pipeline should be dropped entirely
        let filter = NodeFilter::Nodes(["n1"].iter().map(|s| s.to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        let mut top_items = Vec::new();
        filtered.for_each_item(&mut |item| top_items.push((item.name(), item.is_leaf())));
        assert_eq!(top_items, [("n1", true)]);
    }
}
