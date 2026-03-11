//! Types for the pipeline graph.

use std::collections::{HashMap, HashSet};
use std::prelude::v1::*;

use crate::pipeline::{DatasetRef, PipelineInfo};

/// Pre-computed DAG representation of a pipeline, used by the parallel
/// runner and the visualization server.
pub struct PipelineGraph<'a> {
    pub nodes: Vec<GraphNode<'a>>,
    pub edges: Vec<Edge<'a>>,
    pub node_indices: Vec<usize>,
    pub source_datasets: HashSet<usize>,
    pub dataset_names: HashMap<usize, String>,
}

/// A node or pipeline in the graph, with its resolved dataset references.
pub struct GraphNode<'a> {
    pub id: usize,
    pub name: &'static str,
    pub is_pipe: bool,
    pub inputs: Vec<DatasetRef<'a>>,
    pub outputs: Vec<DatasetRef<'a>>,
    pub pipe_children: Vec<usize>,
    pub parent_pipe: Option<usize>,
    pub item: &'a dyn PipelineInfo,
}

/// A data dependency edge: one node's output feeds another node's input.
pub struct Edge<'a> {
    pub from_node: usize,
    pub to_node: usize,
    pub dataset: DatasetRef<'a>,
}
