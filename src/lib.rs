#![feature(unboxed_closures, fn_traits, tuple_trait)]

pub mod core;
pub mod datasets;
pub mod hooks;
pub mod runners;

// Re-export commonly used items
pub use core::{Node, Pipeline, PipelineItem, Steps};
pub use datasets::Dataset;
pub use hooks::{Hook, Hooks};
pub use runners::{ParallelRunner, Runner, SequentialRunner};
