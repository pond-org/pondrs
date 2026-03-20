//! Core types and traits for pipelines.

mod check;
#[cfg(feature = "std")]
mod dyn_steps;
pub(crate) mod id_set;
mod ident;
mod into_result;
mod node;
mod pipeline;
mod pipeline_fn;
#[cfg(feature = "std")]
mod split_join;
pub mod stable;
mod steps;
mod traits;

pub use crate::error::CheckError;
pub use ident::Ident;
pub use into_result::IntoNodeResult;
pub use node::{CompatibleOutput, Node};
pub use pipeline::Pipeline;
pub use pipeline_fn::PipelineFn;
pub use steps::{StepInfo, Steps};
#[cfg(feature = "std")]
pub use dyn_steps::StepVec;
#[cfg(feature = "std")]
pub use split_join::{Split, Join};
#[cfg(feature = "std")]
pub(crate) use traits::ptr_to_id;
pub use traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, PipelineInfo, RunnableStep};
