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

macro_rules! impl_runners {
    ($($R:ident $idx:tt),+) => {
        impl<$($R: Runner),+> Runners for ($($R,)+) {
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
                $(
                    if self.$idx.name() == name {
                        return Some(self.$idx.run(pipe, catalog, params, hooks));
                    }
                )+
                None
            }

            fn for_each_name(&self, f: &mut dyn FnMut(&str)) {
                $(f(self.$idx.name());)+
            }
        }
    };
}

impl_runners!(R0 0);
impl_runners!(R0 0, R1 1);
impl_runners!(R0 0, R1 1, R2 2);
impl_runners!(R0 0, R1 1, R2 2, R3 3);
impl_runners!(R0 0, R1 1, R2 2, R3 3, R4 4);
