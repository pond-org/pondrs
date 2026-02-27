#![no_std]
#![feature(unboxed_closures, fn_traits, tuple_trait)]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(feature = "std")]
pub mod catalog_indexer;
pub mod core;
pub mod datasets;
pub mod error;
#[cfg(feature = "std")]
pub mod graph;
pub mod hooks;
pub mod runners;

// Re-export commonly used items
#[cfg(feature = "std")]
pub use catalog_indexer::{CatalogIndex, index_catalog};
pub use core::{DatasetRef, IntoNodeResult, Node, Pipeline, PipelineInfo, PipelineItem, StepInfo, Steps};
pub use datasets::Dataset;
pub use error::PondError;
#[cfg(feature = "std")]
pub use graph::{PipelineGraph, build_pipeline_graph};
pub use hooks::{Hook, Hooks};
pub use runners::{Runner, SequentialRunner};
#[cfg(feature = "std")]
pub use runners::ParallelRunner;
