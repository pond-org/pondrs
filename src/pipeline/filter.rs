//! Node filtering for partial pipeline execution.

use std::collections::HashSet;
use std::prelude::v1::*;

use serde::Serialize;

use crate::error::PondError;
use crate::graph::build_pipeline_graph;

use super::dyn_steps::DynSteps;
use super::path;
use super::traits::{ptr_to_id, DatasetRef, StepMeta, Group, Step, StepKind};
use super::steps::{StepsMeta, Steps};

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

/// Filter a pipeline's steps, returning a `DynSteps` containing only the
/// nodes that match the filter. Pipeline structure is preserved: sub-pipelines
/// whose children partially match are emitted as `DynPipeline` wrappers.
pub fn filter_steps<'a, E>(
    pipe: &'a impl Steps<E>,
    catalog: &impl Serialize,
    params: &impl Serialize,
    filter: &NodeFilter,
) -> Result<DynSteps<'a, E>, PondError>
where
    E: From<PondError> + Send + Sync + 'static,
{
    let graph = build_pipeline_graph(pipe, catalog, params);

    // Resolve filter to the graph indices of the leaves to keep, then to
    // their ptr-based ids for fast lookup during the tree walk.
    let keep_ids: HashSet<usize> = resolve_keep_set(&graph, filter)?
        .into_iter()
        .map(|i| graph.nodes[i].id)
        .collect();

    // Walk the Steps tree and collect matching items
    let mut result: DynSteps<'a, E> = Vec::new();
    pipe.for_each_step(&mut |item| {
        collect_filtered(item, &keep_ids, &mut result);
    });

    Ok(result)
}

/// Resolve a `NodeFilter` into the graph indices of the leaves to keep.
///
/// A name selects every leaf whose path it matches (see [`path::matches`]):
/// the full path, or any suffix starting at a segment boundary. So
/// `compute` selects `north/compute` and `south/compute` alike, while
/// `north/compute` selects one. A name matching no leaf is an error.
fn resolve_keep_set(
    graph: &crate::graph::PipelineGraph<'_>,
    filter: &NodeFilter,
) -> Result<HashSet<usize>, PondError> {
    match filter {
        NodeFilter::Nodes(names) => {
            let mut keep = HashSet::new();
            for name in names {
                keep.extend(matching_leaves(graph, name)?);
            }
            Ok(keep)
        }
        NodeFilter::FromTo { from, to } => {
            let mut from_seeds = Vec::new();
            for name in from {
                from_seeds.extend(matching_leaves(graph, name)?);
            }
            let mut to_seeds = Vec::new();
            for name in to {
                to_seeds.extend(matching_leaves(graph, name)?);
            }
            Ok(resolve_from_to(graph, &from_seeds, &to_seeds))
        }
    }
}

/// Graph indices of the leaves whose path `name` matches.
fn matching_leaves(
    graph: &crate::graph::PipelineGraph<'_>,
    name: &str,
) -> Result<Vec<usize>, PondError> {
    let found: Vec<usize> = graph
        .node_indices
        .iter()
        .copied()
        .filter(|&i| path::matches(&graph.nodes[i].path, name))
        .collect();
    if found.is_empty() {
        return Err(PondError::NodeNotFound(name.to_string()));
    }
    Ok(found)
}

/// Compute the subgraph between the `from` and `to` seed leaves using edge
/// traversal. An empty seed list leaves that side unconstrained.
fn resolve_from_to(
    graph: &crate::graph::PipelineGraph<'_>,
    from: &[usize],
    to: &[usize],
) -> HashSet<usize> {
    let leaves = &graph.node_indices;

    // Build adjacency lists from edges
    let mut forward: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    let mut backward: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for edge in &graph.edges {
        forward.entry(edge.from_node).or_default().push(edge.to_node);
        backward.entry(edge.to_node).or_default().push(edge.from_node);
    }

    // Forward reachable from `from` nodes (descendants)
    let forward_set = if from.is_empty() {
        leaves.iter().copied().collect::<HashSet<usize>>()
    } else {
        reachable(from, &forward)
    };

    // Backward reachable from `to` nodes (ancestors)
    let backward_set = if to.is_empty() {
        leaves.iter().copied().collect::<HashSet<usize>>()
    } else {
        reachable(to, &backward)
    };

    forward_set.intersection(&backward_set).copied().collect()
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
    item: &'a dyn Step<E>,
    keep_ids: &HashSet<usize>,
    out: &mut DynSteps<'a, E>,
) where
    E: From<PondError> + Send + Sync + 'static,
{
    match item.kind() {
        StepKind::Leaf(_) => {
            let id = ptr_to_id(item as &dyn StepMeta);
            if keep_ids.contains(&id) {
                out.push(Box::new(item));
            }
        }
        StepKind::Group(group) => {
            let mut children: DynSteps<'a, E> = Vec::new();
            group.for_each_child_step(&mut |child| {
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
}

/// A dynamically-constructed pipeline container used by node filtering.
///
/// Mirrors `Pipeline` but stores inputs/outputs as `Vec<DatasetRef>` and
/// children as a `DynSteps`, allowing construction from filtered tree walks.
struct DynPipeline<'a, E> {
    name: &'a str,
    inputs: Vec<DatasetRef<'a>>,
    outputs: Vec<DatasetRef<'a>>,
    steps: DynSteps<'a, E>,
}

impl<E> StepMeta for DynPipeline<'_, E>
where
    E: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn type_string(&self) -> &'static str {
        "pipeline"
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta)) {
        self.steps.for_each_meta(f);
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

impl<E> Group<E> for DynPipeline<'_, E>
where
    E: From<PondError> + Send + Sync + 'static,
{
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>)) {
        self.steps.for_each_step(f);
    }
}

impl<E> Step<E> for DynPipeline<'_, E>
where
    E: From<PondError> + Send + Sync + 'static,
{
    fn kind(&self) -> StepKind<'_, E> { StepKind::Group(self) }
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
    fn leaf_names<'a, E: 'a>(steps: &'a impl Steps<E>) -> Vec<&'a str> {
        let mut names = Vec::new();
        steps.for_each_step(&mut |item| {
            collect_leaf_names(item, &mut names);
        });
        names
    }

    fn collect_leaf_names<'a, E>(item: &'a dyn Step<E>, names: &mut Vec<&'a str>) {
        match item.kind() {
            StepKind::Leaf(_) => {
                names.push(item.name());
            }
            StepKind::Group(group) => {
                group.for_each_child_step(&mut |child| {
                    collect_leaf_names(child, names);
                });
            }
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

        let filter = NodeFilter::Nodes(["n1", "n3"].iter().map(|s| (*s).to_string()).collect());
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
        let filter = NodeFilter::Nodes(["n2"].iter().map(|s| (*s).to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        assert_eq!(leaf_names(&filtered), ["n2"]);

        // Verify pipeline structure is preserved (one top-level item that is not a leaf)
        let mut top_items = Vec::new();
        filtered.for_each_step(&mut |item| top_items.push((item.name(), item.is_leaf())));
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
            from: ["n2"].iter().map(|s| (*s).to_string()).collect(),
            to: ["n3"].iter().map(|s| (*s).to_string()).collect(),
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
            from: ["n2"].iter().map(|s| (*s).to_string()).collect(),
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
            to: ["n2"].iter().map(|s| (*s).to_string()).collect(),
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

        let filter = NodeFilter::Nodes(["nonexistent"].iter().map(|s| (*s).to_string()).collect());
        let result = filter_steps::<PondError>(&pipe, &cat, &params, &filter);

        assert!(matches!(result, Err(PondError::NodeNotFound(ref s)) if s == "nonexistent"));
    }

    /// Two groups each holding a node named `n2`, fed from `n1`.
    fn twin_groups<'a>(cat: &'a Cat, params: &'a Params) -> impl Steps<PondError> + 'a {
        (
            Node { name: "n1", func: |v| (v,), input: (&params.x,), output: (&cat.a,) },
            Pipeline {
                name: "left",
                steps: (Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.b,) },),
                input: (&cat.a,),
                output: (&cat.b,),
            },
            Pipeline {
                name: "right",
                steps: (Node { name: "n2", func: |v| (v,), input: (&cat.a,), output: (&cat.c,) },),
                input: (&cat.a,),
                output: (&cat.c,),
            },
        )
    }

    fn top_level_names<E>(steps: &DynSteps<'_, E>) -> Vec<String> {
        let mut names = Vec::new();
        steps.for_each_step(&mut |item| names.push(item.name().to_string()));
        names
    }

    #[test]
    fn filter_by_local_name_selects_every_match() {
        let cat = Cat { a: MemoryDataset::new(), b: MemoryDataset::new(), c: MemoryDataset::new(), d: MemoryDataset::new() };
        let params = Params { x: Param(1) };
        let pipe = twin_groups(&cat, &params);

        let filter = NodeFilter::Nodes(["n2"].iter().map(|s| (*s).to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();
        assert_eq!(top_level_names(&filtered), ["left", "right"]);
    }

    #[test]
    fn filter_by_path_selects_one() {
        let cat = Cat { a: MemoryDataset::new(), b: MemoryDataset::new(), c: MemoryDataset::new(), d: MemoryDataset::new() };
        let params = Params { x: Param(1) };
        let pipe = twin_groups(&cat, &params);

        let filter = NodeFilter::Nodes(["right/n2"].iter().map(|s| (*s).to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();
        assert_eq!(top_level_names(&filtered), ["right"]);

        // A suffix that does not start at a segment boundary matches nothing.
        let filter = NodeFilter::Nodes(["ight/n2"].iter().map(|s| (*s).to_string()).collect());
        let result = filter_steps::<PondError>(&pipe, &cat, &params, &filter);
        assert!(matches!(result, Err(PondError::NodeNotFound(_))));
    }

    #[test]
    fn filter_from_path() {
        let cat = Cat { a: MemoryDataset::new(), b: MemoryDataset::new(), c: MemoryDataset::new(), d: MemoryDataset::new() };
        let params = Params { x: Param(1) };
        let pipe = twin_groups(&cat, &params);

        let filter = NodeFilter::FromTo {
            from: ["left/n2"].iter().map(|s| (*s).to_string()).collect(),
            to: HashSet::new(),
        };
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();
        assert_eq!(top_level_names(&filtered), ["left"]);
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
        let filter = NodeFilter::Nodes(["n1"].iter().map(|s| (*s).to_string()).collect());
        let filtered = filter_steps::<PondError>(&pipe, &cat, &params, &filter).unwrap();

        let mut top_items = Vec::new();
        filtered.for_each_step(&mut |item| top_items.push((item.name(), item.is_leaf())));
        assert_eq!(top_items, [("n1", true)]);
    }
}
