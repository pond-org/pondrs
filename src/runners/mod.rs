//! Pipeline runners.

#[cfg(feature = "std")]
mod parallel;
mod sequential;

#[cfg(feature = "std")]
pub use parallel::ParallelRunner;
pub use sequential::SequentialRunner;

use serde::Serialize;

use crate::core::Steps;

/// Trait for pipeline runners.
pub trait Runner {
    fn run(&self, pipe: &impl Steps, catalog: &impl Serialize, params: &impl Serialize);
}
