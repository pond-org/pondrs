//! Core types and traits for pipelines.

mod into_result;
mod node;
mod pipeline;
mod steps;
mod traits;

pub use into_result::IntoNodeResult;
pub use node::Node;
pub use pipeline::Pipeline;
pub use steps::{StepInfo, Steps};
pub use traits::{DatasetRef, NodeInput, NodeOutput, PipelineInfo, PipelineItem, ptr_to_id};
