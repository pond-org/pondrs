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
    /// The name used to select this runner (e.g. via CLI `--runner` flag).
    fn name(&self) -> &'static str;

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

/// Trait for a collection of runners (implemented for tuples).
/// Allows selecting a runner by name at runtime.
pub trait Runners {
    fn run_by_name<E>(
        &self,
        name: &str,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Option<Result<(), E>>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static;

    fn for_each_name(&self, f: &mut dyn FnMut(&str));
}

impl<R: Runner> Runners for (R,) {
    fn run_by_name<E>(
        &self,
        name: &str,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Option<Result<(), E>>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        if self.0.name() == name {
            return Some(self.0.run(pipe, catalog, params, hooks));
        }
        None
    }

    fn for_each_name(&self, f: &mut dyn FnMut(&str)) {
        f(self.0.name());
    }
}

impl<R1: Runner, R2: Runner> Runners for (R1, R2) {
    fn run_by_name<E>(
        &self,
        name: &str,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Option<Result<(), E>>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        if self.0.name() == name {
            return Some(self.0.run(pipe, catalog, params, hooks));
        }
        if self.1.name() == name {
            return Some(self.1.run(pipe, catalog, params, hooks));
        }
        None
    }

    fn for_each_name(&self, f: &mut dyn FnMut(&str)) {
        f(self.0.name());
        f(self.1.name());
    }
}

impl<R1: Runner, R2: Runner, R3: Runner> Runners for (R1, R2, R3) {
    fn run_by_name<E>(
        &self,
        name: &str,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Option<Result<(), E>>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        if self.0.name() == name {
            return Some(self.0.run(pipe, catalog, params, hooks));
        }
        if self.1.name() == name {
            return Some(self.1.run(pipe, catalog, params, hooks));
        }
        if self.2.name() == name {
            return Some(self.2.run(pipe, catalog, params, hooks));
        }
        None
    }

    fn for_each_name(&self, f: &mut dyn FnMut(&str)) {
        f(self.0.name());
        f(self.1.name());
        f(self.2.name());
    }
}
