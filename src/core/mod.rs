//! Core types and traits for pipelines.

mod check;
pub(crate) mod id_set;
mod into_result;
mod node;
mod pipeline;
mod steps;
mod traits;

pub use check::CheckError;
pub use into_result::IntoNodeResult;
pub use node::Node;
pub use pipeline::Pipeline;
pub use steps::{StepInfo, Steps};
pub use traits::{DatasetEvent, DatasetInfo, DatasetRef, NodeInput, NodeOutput, PipelineInfo, PipelineItem, ptr_to_id};
