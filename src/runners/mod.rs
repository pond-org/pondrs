//! Pipeline runners.

mod parallel;
mod sequential;

pub use parallel::ParallelRunner;
pub use sequential::SequentialRunner;

use crate::core::Steps;

/// Trait for pipeline runners.
pub trait Runner {
    fn run(&self, pipe: &impl Steps);
}
