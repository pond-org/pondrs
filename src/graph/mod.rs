//! Pipeline graph: a pre-computed DAG representation of the pipeline.

mod build;
mod types;
mod validate;

pub use build::build_pipeline_graph;
pub use types::{Edge, GraphNode, PipelineGraph};
pub use validate::ValidationError;
