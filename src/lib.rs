//! A pipeline execution library for building data processing workflows.
//!
//! Pipelines are composed of [`Node`]s that read from and write to [`Dataset`]s.
//! Nodes are grouped into [`Steps`] (tuples) and executed by a [`Runner`].
//! The [`App`] struct ties everything together with CLI dispatch, YAML config
//! loading, and hook-based lifecycle events.
//!
//! Works in `no_std` environments (with `CellDataset` + `SequentialRunner`)
//! and scales up to parallel execution with `std`.

#![no_std]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

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
pub use app::App;
#[cfg(feature = "std")]
pub use catalog_indexer::{CatalogIndex, index_catalog, index_catalog_with_params};
pub use datasets::{Dataset, DatasetMeta};
#[cfg(feature = "std")]
pub use datasets::TemplatedCatalog;
pub use error::{CheckError, PondError};
#[cfg(feature = "std")]
pub use graph::{PipelineGraph, build_pipeline_graph};
pub use hooks::{Hook, Hooks};
pub use pipeline::{
    DatasetEvent, DatasetRef, Ident, IntoNodeResult, Node, Pipeline, PipelineFn, StepInfo,
    RunnableStep, PipelineInfo, Steps,
};
#[cfg(feature = "std")]
pub use pipeline::{StepVec, Split, Join};
#[cfg(feature = "std")]
pub use runners::ParallelRunner;
pub use runners::{Runner, Runners, SequentialRunner};
