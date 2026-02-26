#![feature(unboxed_closures, fn_traits, tuple_trait)]

pub mod catalog_indexer;
pub mod core;
pub mod datasets;
pub mod graph;
pub mod hooks;
pub mod runners;

// Re-export commonly used items
pub use catalog_indexer::{CatalogIndex, index_catalog};
pub use core::{DatasetRef, Node, Pipeline, PipelineItem, Steps};
pub use datasets::Dataset;
pub use graph::{PipelineGraph, build_pipeline_graph};
pub use hooks::{Hook, Hooks};
pub use runners::{ParallelRunner, Runner, SequentialRunner};
