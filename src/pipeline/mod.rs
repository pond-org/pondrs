//! Core types and traits for pipelines.

mod check;
pub(crate) mod id_set;
mod into_result;
mod node;
mod pipeline;
mod steps;
mod traits;

pub use crate::error::CheckError;
pub use into_result::IntoNodeResult;
pub use node::Node;
pub use pipeline::Pipeline;
pub use steps::{StepInfo, Steps};
pub use traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, PipelineInfo, RunnableStep};
#[cfg(feature = "std")]
pub(crate) use traits::ptr_to_id;
