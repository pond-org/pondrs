//! Error types for the pipeline framework.

use thiserror::Error;

/// Framework-level errors from dataset I/O and pipeline infrastructure.
///
/// Feature-gated variants are only available when the corresponding feature
/// is enabled. The `DatasetNotLoaded` variant is always available (`no_std`).
///
/// Marked `#[non_exhaustive]`: which variants exist already depends on the
/// feature set a build resolves to, so downstream code must not rely on
/// matching them exhaustively.
#[derive(Error, Debug)]
#[non_exhaustive]
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

    #[error("Hook aborted: {0}")]
    HookAbort(&'static str),

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

    /// A foreign error, preserved whole: `Display`, `source()` chain, and
    /// `downcast_ref` all keep working. Prefer this over [`Custom`](Self::Custom),
    /// which flattens an error to its message.
    #[cfg(feature = "std")]
    #[error(transparent)]
    Other(#[from] std::boxed::Box<dyn core::error::Error + Send + Sync>),

    /// A static message. The `no_std`-usable counterpart of
    /// [`Custom`](Self::Custom) and [`Other`](Self::Other), which both need an
    /// allocator.
    #[error("{0}")]
    Message(&'static str),

    #[cfg(feature = "std")]
    #[error("Key mismatch: expected keys {expected:?}, got {actual:?}")]
    KeyMismatch {
        expected: std::vec::Vec<std::string::String>,
        actual: std::vec::Vec<std::string::String>,
    },

    #[cfg(feature = "std")]
    #[error("Node not found: '{0}'")]
    NodeNotFound(std::string::String),
}

impl PondError {
    /// Wrap a foreign error in [`Other`](Self::Other), keeping it whole.
    #[cfg(feature = "std")]
    pub fn other<E: core::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self::Other(std::boxed::Box::new(e))
    }
}

/// Validation error from [`StepsMeta::check`](crate::pipeline::StepsMeta::check).
///
/// Borrows the node and pipeline names from the checked steps, so it lives no
/// longer than the pipeline it describes.
#[derive(Debug)]
pub enum CheckError<'a> {
    /// A node reads a dataset that is produced by a later node (wrong order).
    InputNotProduced {
        node_name: &'a str,
        dataset_id: usize,
    },
    /// A dataset is produced by more than one node.
    DuplicateOutput {
        node_name: &'a str,
        dataset_id: usize,
    },
    /// A node writes to a param dataset (params are read-only).
    ParamWritten {
        node_name: &'a str,
        dataset_id: usize,
    },
    /// A pipeline declares an input that none of its children consume.
    UnusedPipelineInput {
        pipeline_name: &'a str,
        dataset_id: usize,
    },
    /// A pipeline declares an output that none of its children produce.
    UnproducedPipelineOutput {
        pipeline_name: &'a str,
        dataset_id: usize,
    },
    /// A child node consumes an external dataset not declared in the pipeline's inputs.
    UndeclaredPipelineInput {
        pipeline_name: &'a str,
        dataset_id: usize,
    },
    /// Two datasets of different types share one pointer id — the DAG would
    /// silently merge them. Usually caused by zero-sized dataset types.
    AliasedDatasets {
        node_name: &'a str,
        dataset_id: usize,
        type_name: &'static str,
        conflicting_type_name: &'static str,
    },
    /// Two steps in one group (or at the top level, `group: None`) share a
    /// name, so their paths would coincide and hooks, the cache, CLI node
    /// filters and viz could not tell them apart.
    DuplicateStepName {
        group: Option<&'a str>,
        name: &'a str,
    },
    /// A step name contains `/`, the step path separator.
    InvalidStepName {
        name: &'a str,
    },
    /// The fixed-capacity dataset buffer overflowed.
    CapacityExceeded,
}

impl core::fmt::Display for CheckError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputNotProduced { node_name, dataset_id } => {
                write!(f, "Node '{node_name}' requires dataset {dataset_id:#x}, which is produced by a later node")
            }
            Self::DuplicateOutput { node_name, dataset_id } => {
                write!(f, "Node '{node_name}' produces dataset {dataset_id:#x}, which was already produced by an earlier node")
            }
            Self::ParamWritten { node_name, dataset_id } => {
                write!(f, "Node '{node_name}' writes to param dataset {dataset_id:#x}, but params are read-only")
            }
            Self::UnusedPipelineInput { pipeline_name, dataset_id } => {
                write!(f, "Pipeline '{pipeline_name}' declares input {dataset_id:#x}, but none of its children consume it")
            }
            Self::UnproducedPipelineOutput { pipeline_name, dataset_id } => {
                write!(f, "Pipeline '{pipeline_name}' declares output {dataset_id:#x}, but none of its children produce it")
            }
            Self::UndeclaredPipelineInput { pipeline_name, dataset_id } => {
                write!(f, "Pipeline '{pipeline_name}' has a child that consumes external dataset {dataset_id:#x}, which is not declared in the pipeline's inputs")
            }
            Self::AliasedDatasets { node_name, dataset_id, type_name, conflicting_type_name } => {
                write!(f, "Node '{node_name}' uses dataset {dataset_id:#x} of type `{type_name}`, but `{conflicting_type_name}` was already seen at that same address; the two datasets would be merged into one graph node (a zero-sized dataset type is the usual cause)")
            }
            Self::DuplicateStepName { group: Some(group), name } => {
                write!(f, "Pipeline '{group}' has more than one step named '{name}'; sibling steps need distinct names")
            }
            Self::DuplicateStepName { group: None, name } => {
                write!(f, "More than one top-level step is named '{name}'; sibling steps need distinct names")
            }
            Self::InvalidStepName { name } => {
                write!(f, "Step name '{name}' contains '/', which separates the segments of a step path")
            }
            Self::CapacityExceeded => {
                write!(f, "Dataset capacity exceeded; use check_with_capacity::<N>() with a larger N")
            }
        }
    }
}

/// Non-fatal diagnostic from
/// [`StepsMeta::for_each_warning`](crate::pipeline::StepsMeta::for_each_warning).
///
/// Warnings describe catalogs that will *work* but whose datasets cannot be
/// identified or named reliably. They never fail a `check`.
#[non_exhaustive]
#[derive(Debug)]
pub enum CheckWarning<'a> {
    /// Zero-sized dataset type: its address is not a reliable identity and may
    /// collide with a sibling field.
    ZeroSizedDataset {
        node_name: &'a str,
        dataset_id: usize,
        type_name: &'static str,
    },

    /// Type ident does not follow the `*Dataset` / `Param` convention, so the
    /// (std) catalog indexer will recurse into it and resolve an interior field
    /// name instead of the dataset's own name in logs and viz.
    ///
    /// Emitted on `no_std` too, where no indexer exists: the same catalog is
    /// typically also built for host tooling and viz, and the convention is a
    /// property of the type, not of the build.
    UnconventionalDatasetType {
        node_name: &'a str,
        dataset_id: usize,
        type_name: &'static str,
    },
}

impl core::fmt::Display for CheckWarning<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroSizedDataset { node_name, dataset_id, type_name } => {
                write!(f, "dataset type `{type_name}` used by node '{node_name}' is zero-sized; its address ({dataset_id:#x}) is not a reliable identity and may collide with a sibling catalog field")
            }
            Self::UnconventionalDatasetType { node_name, dataset_id, type_name } => {
                write!(f, "dataset type `{type_name}` used by node '{node_name}' does not end in `Dataset`; the catalog indexer will recurse into it and resolve an interior field name for dataset {dataset_id:#x} in logs and viz")
            }
        }
    }
}

impl From<core::convert::Infallible> for PondError {
    fn from(x: core::convert::Infallible) -> Self {
        match x {}
    }
}

impl From<crate::hooks::HookAbort> for PondError {
    fn from(e: crate::hooks::HookAbort) -> Self {
        Self::HookAbort(e.0)
    }
}
