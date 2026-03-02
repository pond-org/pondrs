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
use crate::hooks::Hooks;

/// Trait for pipeline runners.
pub trait Runner {
    fn run<E>(
        &self,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static;
}

/// Zero-sized runner that disables a runner slot.
/// Used with `None::<NoRunner>` to prevent monomorphization of unused runners.
pub struct NoRunner;

impl Runner for NoRunner {
    fn run<E>(
        &self,
        _pipe: &impl Steps<E>,
        _catalog: &impl Serialize,
        _params: &impl Serialize,
        _hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        unreachable!("NoRunner should never be called")
    }
}
