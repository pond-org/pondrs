//! Owned serializable types for the pipeline visualization graph.

use std::collections::HashMap;
use std::prelude::v1::*;

use serde::Serialize;

use crate::graph::PipelineGraph;

/// Owned, serializable representation of the full pipeline graph.
#[derive(Serialize, Clone)]
pub struct VizGraph {
    pub name: String,
    pub nodes: Vec<VizNode>,
    pub edges: Vec<VizEdge>,
    pub datasets: Vec<VizDataset>,
}

/// Serializable representation of a pipeline node or sub-pipeline.
#[derive(Serialize, Clone)]
pub struct VizNode {
    /// Index into `VizGraph::nodes`.
    pub id: usize,
    pub name: String,
    pub type_string: String,
    pub is_pipe: bool,
    pub parent_pipe: Option<usize>,
    pub pipe_children: Vec<usize>,
    /// Dataset ptr IDs used as inputs.
    pub input_dataset_ids: Vec<usize>,
    /// Dataset ptr IDs produced as outputs.
    pub output_dataset_ids: Vec<usize>,
}

/// Serializable metadata for a dataset in the visualization.
#[derive(Serialize, Clone)]
pub struct VizDataset {
    /// Dataset ptr ID (from `ptr_to_id`).
    pub id: usize,
    pub name: String,
    pub type_string: String,
    pub is_param: bool,
    pub has_html: bool,
    pub has_yaml: bool,
}

/// Serializable edge connecting two nodes through a dataset.
#[derive(Serialize, Clone)]
pub struct VizEdge {
    /// Index into `VizGraph::nodes` for the producing node.
    pub from_node: usize,
    /// Index into `VizGraph::nodes` for the consuming node.
    pub to_node: usize,
    pub dataset_id: usize,
}

/// Convert a `PipelineGraph` into an owned, serializable `VizGraph`.
pub fn viz_graph_from(graph: &PipelineGraph<'_>) -> VizGraph {
    let mut datasets: Vec<VizDataset> = Vec::new();
    let mut dataset_map: HashMap<usize, usize> = HashMap::new(); // ptr_id → index

    // Collect all unique datasets from node inputs and outputs
    for node in &graph.nodes {
        for ds in node.inputs.iter().chain(node.outputs.iter()) {
            if dataset_map.contains_key(&ds.id) {
                continue;
            }
            let idx = datasets.len();
            let name = graph
                .dataset_names
                .get(&ds.id)
                .cloned()
                .unwrap_or_else(|| format!("dataset_{}", ds.id));
            datasets.push(VizDataset {
                id: ds.id,
                name,
                type_string: ds.meta.type_string().to_string(),
                is_param: ds.meta.is_param(),
                has_html: ds.meta.html().is_some(),
                has_yaml: ds.meta.yaml().is_some(),
            });
            dataset_map.insert(ds.id, idx);
        }
    }

    let nodes: Vec<VizNode> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| VizNode {
            id: i,
            name: n.name.to_string(),
            type_string: n.item.type_string().to_string(),
            is_pipe: n.is_pipe,
            parent_pipe: n.parent_pipe,
            pipe_children: n.pipe_children.clone(),
            input_dataset_ids: n.inputs.iter().map(|d| d.id).collect(),
            output_dataset_ids: n.outputs.iter().map(|d| d.id).collect(),
        })
        .collect();

    let edges: Vec<VizEdge> = graph
        .edges
        .iter()
        .map(|e| VizEdge {
            from_node: e.from_node,
            to_node: e.to_node,
            dataset_id: e.dataset.id,
        })
        .collect();

    VizGraph { name: String::new(), nodes, edges, datasets }
}

/// Collect `DatasetMeta` references for all datasets in the graph, erasing
/// their borrow lifetime to `'static`.
///
/// Returns a map from dataset ptr ID to `&'static dyn DatasetMeta`.
///
/// # Safety
/// The returned references are valid only as long as the catalog that owns
/// the datasets remains alive. This is upheld because `start_server` blocks
/// and the originating catalog lives on the calling stack frame for the
/// entire server lifetime.
pub fn collect_dataset_meta(
    graph: &PipelineGraph<'_>,
) -> HashMap<usize, &'static dyn crate::DatasetMeta> {
    let mut map: HashMap<usize, &'static dyn crate::DatasetMeta> = HashMap::new();
    for node in &graph.nodes {
        for ds in node.inputs.iter().chain(node.outputs.iter()) {
            if map.contains_key(&ds.id) {
                continue;
            }
            // SAFETY: The DatasetMeta objects live for the entire server
            // lifetime because start_server() blocks while the catalog is
            // alive on the calling stack frame.
            let static_ref: &'static dyn crate::DatasetMeta = unsafe {
                std::mem::transmute::<&dyn crate::DatasetMeta, &'static dyn crate::DatasetMeta>(
                    ds.meta,
                )
            };
            map.insert(ds.id, static_ref);
        }
    }
    map
}
