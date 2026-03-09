//! Error types for the pipeline framework.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PondError {
    #[cfg(feature = "std")]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "polars")]
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    #[cfg(feature = "yaml")]
    #[error("YAML parse error: {0}")]
    YamlScan(#[from] yaml_rust2::ScanError),

    #[cfg(feature = "yaml")]
    #[error("YAML emit error: {0}")]
    YamlEmit(#[from] yaml_rust2::EmitError),

    #[cfg(feature = "std")]
    #[error("Serde YAML error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    #[cfg(any(feature = "json", feature = "plotly", feature = "viz"))]
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(feature = "image")]
    #[error("Image error: {0}")]
    Image(#[from] ::image::ImageError),

    #[error("Dataset not loaded: no data available")]
    DatasetNotLoaded,

    #[error("Runner not found")]
    RunnerNotFound,

    #[error("Pipeline check failed")]
    CheckFailed,

    #[cfg(feature = "std")]
    #[error("Lock poisoned: {0}")]
    LockPoisoned(std::string::String),

    #[cfg(feature = "std")]
    #[error("{0}")]
    Custom(std::string::String),
}

/// Validation error from [`StepInfo::check`](crate::pipeline::StepInfo::check).
#[derive(Debug)]
pub enum CheckError {
    /// A node reads a dataset that is produced by a later node (wrong order).
    InputNotProduced {
        node_name: &'static str,
        dataset_id: usize,
    },
    /// A dataset is produced by more than one node.
    DuplicateOutput {
        node_name: &'static str,
        dataset_id: usize,
    },
    /// A node writes to a param dataset (params are read-only).
    ParamWritten {
        node_name: &'static str,
        dataset_id: usize,
    },
    /// A pipeline declares an input that none of its children consume.
    UnusedPipelineInput {
        pipeline_name: &'static str,
        dataset_id: usize,
    },
    /// A pipeline declares an output that none of its children produce.
    UnproducedPipelineOutput {
        pipeline_name: &'static str,
        dataset_id: usize,
    },
    /// The fixed-capacity dataset buffer overflowed.
    CapacityExceeded,
}

impl core::fmt::Display for CheckError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CheckError::InputNotProduced { node_name, dataset_id } => {
                write!(f, "Node '{node_name}' requires dataset {dataset_id:#x}, which is produced by a later node")
            }
            CheckError::DuplicateOutput { node_name, dataset_id } => {
                write!(f, "Node '{node_name}' produces dataset {dataset_id:#x}, which was already produced by an earlier node")
            }
            CheckError::ParamWritten { node_name, dataset_id } => {
                write!(f, "Node '{node_name}' writes to param dataset {dataset_id:#x}, but params are read-only")
            }
            CheckError::UnusedPipelineInput { pipeline_name, dataset_id } => {
                write!(f, "Pipeline '{pipeline_name}' declares input {dataset_id:#x}, but none of its children consume it")
            }
            CheckError::UnproducedPipelineOutput { pipeline_name, dataset_id } => {
                write!(f, "Pipeline '{pipeline_name}' declares output {dataset_id:#x}, but none of its children produce it")
            }
            CheckError::CapacityExceeded => {
                write!(f, "Dataset capacity exceeded; use check_with_capacity::<N>() with a larger N")
            }
        }
    }
}

impl From<core::convert::Infallible> for PondError {
    fn from(x: core::convert::Infallible) -> Self {
        match x {}
    }
}
