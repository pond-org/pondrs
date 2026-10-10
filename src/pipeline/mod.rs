//! Core types and traits for pipelines.

mod check;
#[cfg(feature = "std")]
mod dyn_steps;
#[cfg(feature = "std")]
mod filter;
pub(crate) mod id_set;
mod alias;
mod into_result;
mod node;
#[cfg(feature = "std")]
mod partitioned_node;
#[cfg(feature = "std")]
pub(crate) mod path;
// `pipeline::pipeline` holds the `Pipeline` struct, which is re-exported below;
// the doubled path is never written by hand.
#[allow(clippy::module_inception, reason = "private module, re-exported below")]
mod pipeline;
mod pipeline_fn;
#[cfg(feature = "std")]
mod each_field;
#[cfg(feature = "std")]
mod recurrent;
pub mod stable;
mod steps;
mod traits;

pub use crate::error::{CheckError, CheckWarning};
pub use alias::Alias;
pub use into_result::IntoNodeResult;
pub use node::{CompatibleOutput, Node};
pub use pipeline::Pipeline;
pub use pipeline_fn::PipelineFn;
pub use steps::{StepsMeta, Steps};
#[cfg(feature = "std")]
pub use dyn_steps::DynSteps;
#[cfg(feature = "std")]
pub use filter::{NodeFilter, filter_steps};
#[cfg(feature = "std")]
pub use partitioned_node::PartitionedNode;
#[cfg(feature = "std")]
pub use each_field::EachField;
#[cfg(feature = "std")]
pub use recurrent::{RecurrentNode, RecurrentPipeline, Unrolled};
#[cfg(feature = "std")]
pub(crate) use traits::ptr_to_id;
pub use traits::{DatasetEvent, DatasetRef, DatasetInput, DatasetOutput, NodeInput, NodeInputMeta, NodeOutput, NodeOutputMeta, StepMeta, Leaf, Group, StepKind, Step};
