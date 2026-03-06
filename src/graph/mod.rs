//! Pipeline graph: a pre-computed DAG representation of the pipeline.

mod build;
mod types;

pub use build::build_pipeline_graph;
pub use types::{Edge, GraphNode, PipelineGraph};
