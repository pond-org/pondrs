#![no_std]
#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(feature = "std")]
pub mod app;
#[cfg(feature = "std")]
pub mod catalog_indexer;
pub mod datasets;
pub mod error;
#[cfg(feature = "std")]
pub mod graph;
pub mod hooks;
pub mod pipeline;
pub mod runners;
#[cfg(feature = "viz")]
pub mod viz;

// Re-export commonly used items
#[cfg(feature = "std")]
pub use app::PondApp;
#[cfg(feature = "std")]
pub use catalog_indexer::{CatalogIndex, index_catalog, index_catalog_with_params};
pub use datasets::{Dataset, DatasetMeta};
pub use error::{CheckError, PondError};
#[cfg(feature = "std")]
pub use graph::{PipelineGraph, build_pipeline_graph};
pub use hooks::{Hook, Hooks};
pub use pipeline::{
    DatasetEvent, DatasetRef, Ident, IntoNodeResult, Node, Pipeline, PipelineInfo, RunnableStep,
    StepInfo, Steps,
};
#[cfg(feature = "std")]
pub use runners::ParallelRunner;
pub use runners::{Runner, Runners, SequentialRunner};
