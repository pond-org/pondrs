//! Core types and traits for pipelines.

mod node;
mod pipeline;
mod steps;
mod traits;

pub use node::Node;
pub use pipeline::Pipeline;
pub use steps::Steps;
pub use traits::{DatasetRef, NodeInput, NodeOutput, PipelineItem, ptr_to_id};
