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

    #[cfg(any(feature = "plotly", feature = "viz"))]
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Dataset not loaded: no data available")]
    DatasetNotLoaded,

    #[cfg(feature = "std")]
    #[error("Lock poisoned: {0}")]
    LockPoisoned(std::string::String),

    #[cfg(feature = "std")]
    #[error("{0}")]
    Custom(std::string::String),
}

impl From<core::convert::Infallible> for PondError {
    fn from(x: core::convert::Infallible) -> Self {
        match x {}
    }
}
