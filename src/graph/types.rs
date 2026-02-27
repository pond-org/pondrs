//! Types for the pipeline graph.

use std::collections::{HashMap, HashSet};
use std::prelude::v1::*;

use crate::core::{DatasetRef, PipelineInfo};

pub struct PipelineGraph<'a> {
    pub nodes: Vec<GraphNode<'a>>,
    pub edges: Vec<Edge>,
    pub node_indices: Vec<usize>,
    pub source_datasets: HashSet<usize>,
    pub dataset_names: HashMap<usize, String>,
}

pub struct GraphNode<'a> {
    pub id: usize,
    pub name: &'static str,
    pub is_pipe: bool,
    pub inputs: Vec<DatasetRef>,
    pub outputs: Vec<DatasetRef>,
    pub pipe_children: Vec<usize>,
    pub parent_pipe: Option<usize>,
    pub item: &'a dyn PipelineInfo,
}

pub struct Edge {
    pub from_node: usize,
    pub to_node: usize,
    pub dataset: DatasetRef,
}
