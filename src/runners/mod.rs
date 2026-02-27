//! Pipeline runners.

#[cfg(feature = "std")]
mod parallel;
mod sequential;

#[cfg(feature = "std")]
pub use parallel::ParallelRunner;
pub use sequential::SequentialRunner;

use serde::Serialize;

use crate::core::Steps;
use crate::error::PondError;

/// Trait for pipeline runners.
pub trait Runner {
    fn run<E>(&self, pipe: &impl Steps<E>, catalog: &impl Serialize, params: &impl Serialize) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static;
}
