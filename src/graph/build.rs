//! Pipeline graph construction.

use std::prelude::v1::*;
use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::catalog_indexer::index_catalog;
use crate::core::{PipelineInfo, StepInfo, ptr_to_id};

use super::types::{Edge, GraphNode, PipelineGraph};

/// Internal wrapper to index both catalog and params in one pass.
#[derive(Serialize)]
struct Context<'a, C: Serialize, P: Serialize> {
    catalog: &'a C,
    params: &'a P,
}

pub fn build_pipeline_graph<'a>(
    pipe: &'a impl StepInfo,
    catalog: &impl Serialize,
    params: &impl Serialize,
) -> PipelineGraph<'a> {
    // 1. Build dataset name index from catalog + params
    let context = Context { catalog, params };
    let catalog_index = index_catalog(&context);
    let dataset_names = catalog_index.into_inner();

    // 2. Walk tree, collect all nodes with parent/child relationships
    let mut nodes: Vec<GraphNode<'a>> = Vec::new();
    pipe.for_each_info(&mut |item| {
        collect_node(item, None, &mut nodes);
    });

    // 3. Build edges: match producer outputs to consumer inputs across leaves
    let edges = build_edges(&nodes);

    // 4. Pre-compute node indices (non-pipe items)
    let node_indices: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.is_pipe)
        .map(|(i, _)| i)
        .collect();

    // 5. Pre-compute source datasets (inputs not produced by any node)
    let all_outputs: HashSet<usize> = nodes
        .iter()
        .filter(|n| !n.is_pipe)
        .flat_map(|n| n.outputs.iter().map(|d| d.id))
        .collect();
    let all_inputs: HashSet<usize> = nodes
        .iter()
        .filter(|n| !n.is_pipe)
        .flat_map(|n| n.inputs.iter().map(|d| d.id))
        .collect();
    let source_datasets = all_inputs.difference(&all_outputs).copied().collect();

    PipelineGraph {
        nodes,
        edges,
        node_indices,
        source_datasets,
        dataset_names,
    }
}

fn collect_node<'a>(
    item: &'a dyn PipelineInfo,
    parent: Option<usize>,
    nodes: &mut Vec<GraphNode<'a>>,
) {
    let index = nodes.len();

    let is_pipe = !item.is_leaf();

    let mut inputs = Vec::new();
    item.for_each_input_id(&mut |d| inputs.push(d.clone()));
    let mut outputs = Vec::new();
    item.for_each_output_id(&mut |d| outputs.push(d.clone()));

    nodes.push(GraphNode {
        id: ptr_to_id(item),
        name: item.get_name(),
        is_pipe,
        inputs,
        outputs,
        pipe_children: Vec::new(),
        parent_pipe: parent,
        item,
    });

    // Record this node as a child of its parent pipe
    if let Some(parent_idx) = parent {
        nodes[parent_idx].pipe_children.push(index);
    }

    // Recurse into children
    if is_pipe {
        item.for_each_child(&mut |child| {
            collect_node(child, Some(index), nodes);
        });
    }
}

fn build_edges(nodes: &[GraphNode]) -> Vec<Edge> {
    // Build a map: dataset_id -> producer node index (for leaves only)
    let mut producers: HashMap<usize, usize> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if !node.is_pipe {
            for output in &node.outputs {
                producers.insert(output.id, i);
            }
        }
    }

    // For each leaf's input, if there's a producer, create an edge
    let mut edges = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if !node.is_pipe {
            for input in &node.inputs {
                if let Some(&producer_idx) = producers.get(&input.id) {
                    edges.push(Edge {
                        from_node: producer_idx,
                        to_node: i,
                        dataset: input.clone(),
                    });
                }
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{MemoryDataset, Param};
    use crate::core::{Node, Pipeline};
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestCatalog {
        a: MemoryDataset<i32>,
        b: MemoryDataset<i32>,
        c: MemoryDataset<i32>,
    }

    #[derive(Serialize)]
    struct TestParams {
        initial_value: Param<i32>,
    }

    #[test]
    fn test_linear_pipeline() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams {
            initial_value: Param(1),
        };
        // param -> a -> b -> c
        let pipe = (
            Node {
                name: "n1",
                func: |v| (v,),
                input: (&params.initial_value,),
                output: (&catalog.a,),
            },
            Node {
                name: "n2",
                func: |v| (v,),
                input: (&catalog.a,),
                output: (&catalog.b,),
            },
            Node {
                name: "n3",
                func: |v| (v,),
                input: (&catalog.b,),
                output: (&catalog.c,),
            },
        );

        let graph = build_pipeline_graph(&pipe, &catalog, &params);

        // All 3 are leaves
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.node_indices.len(), 3);
        assert!(graph.nodes.iter().all(|n| !n.is_pipe));

        // Names
        assert_eq!(graph.nodes[0].name, "n1");
        assert_eq!(graph.nodes[1].name, "n2");
        assert_eq!(graph.nodes[2].name, "n3");

        // 2 edges: n1->n2 (via a), n2->n3 (via b)
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from_node, 0);
        assert_eq!(graph.edges[0].to_node, 1);
        assert_eq!(graph.edges[1].from_node, 1);
        assert_eq!(graph.edges[1].to_node, 2);

        // Source datasets: only initial_value (a param)
        assert_eq!(graph.source_datasets.len(), 1);
        let source_id = *graph.source_datasets.iter().next().unwrap();
        assert_eq!(
            graph.dataset_names.get(&source_id).map(|s| s.as_str()),
            Some("params.initial_value")
        );

        // Dataset names contain catalog and params entries
        assert!(graph.dataset_names.values().any(|n| n == "catalog.a"));
        assert!(graph.dataset_names.values().any(|n| n == "catalog.b"));
        assert!(graph.dataset_names.values().any(|n| n == "catalog.c"));

        // n1's input is a param
        assert!(graph.nodes[0].inputs[0].is_param);
        // n2's input is not a param
        assert!(!graph.nodes[1].inputs[0].is_param);
    }

    #[test]
    fn test_diamond_pipeline() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams {
            initial_value: Param(1),
        };
        // param -> a (n1), param -> b (n2), a+b -> c (n3)
        let pipe = (
            Node {
                name: "n1",
                func: |v| (v,),
                input: (&params.initial_value,),
                output: (&catalog.a,),
            },
            Node {
                name: "n2",
                func: |v| (v,),
                input: (&params.initial_value,),
                output: (&catalog.b,),
            },
            Node {
                name: "n3",
                func: |a, b| (a + b,),
                input: (&catalog.a, &catalog.b),
                output: (&catalog.c,),
            },
        );

        let graph = build_pipeline_graph(&pipe, &catalog, &params);

        assert_eq!(graph.nodes.len(), 3);
        // 2 edges into n3: from n1 (via a) and from n2 (via b)
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges.iter().all(|e| e.to_node == 2));
        let from_nodes: HashSet<_> = graph.edges.iter().map(|e| e.from_node).collect();
        assert!(from_nodes.contains(&0));
        assert!(from_nodes.contains(&1));
    }

    #[test]
    fn test_nested_pipeline() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams {
            initial_value: Param(1),
        };
        // param -> a (n1), then Pipeline{ a -> b (n2), b -> c (n3) }
        let pipe = (
            Node {
                name: "n1",
                func: |v| (v,),
                input: (&params.initial_value,),
                output: (&catalog.a,),
            },
            Pipeline {
                name: "inner",
                steps: (
                    Node {
                        name: "n2",
                        func: |v| (v,),
                        input: (&catalog.a,),
                        output: (&catalog.b,),
                    },
                    Node {
                        name: "n3",
                        func: |v| (v,),
                        input: (&catalog.b,),
                        output: (&catalog.c,),
                    },
                ),
                input: (&catalog.a,),
                output: (&catalog.c,),
            },
        );

        let graph = build_pipeline_graph(&pipe, &catalog, &params);

        // 4 nodes: n1, inner (pipeline), n2, n3
        assert_eq!(graph.nodes.len(), 4);

        // 3 leaves: n1, n2, n3
        assert_eq!(graph.node_indices.len(), 3);

        // inner pipeline node
        let inner = &graph.nodes[1];
        assert_eq!(inner.name, "inner");
        assert!(inner.is_pipe);
        assert_eq!(inner.pipe_children.len(), 2); // n2 and n3
        assert!(inner.parent_pipe.is_none()); // top-level

        // n2 and n3 have inner as parent pipe
        let n2 = &graph.nodes[2];
        let n3 = &graph.nodes[3];
        assert_eq!(n2.parent_pipe, Some(1));
        assert_eq!(n3.parent_pipe, Some(1));

        // 2 edges: n1->n2 (via a), n2->n3 (via b)
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_no_output_node() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams {
            initial_value: Param(1),
        };
        // param -> a, a -> print (no output)
        let pipe = (
            Node {
                name: "n1",
                func: |v| (v,),
                input: (&params.initial_value,),
                output: (&catalog.a,),
            },
            Node {
                name: "n2",
                func: |v| println!("{v}"),
                input: (&catalog.a,),
                output: (),
            },
        );

        let graph = build_pipeline_graph(&pipe, &catalog, &params);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.nodes[1].outputs.len(), 0);
    }
}
