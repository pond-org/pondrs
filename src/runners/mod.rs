//! Pipeline runners.

#[cfg(feature = "std")]
mod parallel;
mod sequential;

#[cfg(feature = "std")]
pub use parallel::ParallelRunner;
pub use sequential::SequentialRunner;

use serde::Serialize;

use crate::pipeline::{DatasetEvent, DatasetRef, StepInfo, Steps};
use crate::error::PondError;
use crate::hooks::Hooks;

/// Resolve dataset name from the catalog index and dispatch to hooks.
#[cfg(feature = "std")]
pub(crate) fn dispatch_dataset_event(
    item: &dyn StepInfo,
    ds: &DatasetRef<'_>,
    event: DatasetEvent,
    names: &std::collections::HashMap<usize, std::string::String>,
    hooks: &impl Hooks,
) {
    let ds = DatasetRef { name: names.get(&ds.id).map(|s: &std::string::String| s.as_str()), ..*ds };
    dispatch_dataset_event_raw(item, &ds, event, hooks);
}

/// Dispatch a dataset event to all hooks without name resolution.
pub(crate) fn dispatch_dataset_event_raw(
    item: &dyn StepInfo,
    ds: &DatasetRef<'_>,
    event: DatasetEvent,
    hooks: &impl Hooks,
) {
    match event {
        DatasetEvent::BeforeLoad => hooks.for_each_hook(&mut |h| h.before_dataset_loaded(item, ds)),
        DatasetEvent::AfterLoad => hooks.for_each_hook(&mut |h| h.after_dataset_loaded(item, ds)),
        DatasetEvent::BeforeSave => hooks.for_each_hook(&mut |h| h.before_dataset_saved(item, ds)),
        DatasetEvent::AfterSave => hooks.for_each_hook(&mut |h| h.after_dataset_saved(item, ds)),
    }
}

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
    /// The name of the first (default) runner in the collection.
    fn first_name(&self) -> &'static str;

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
            fn first_name(&self) -> &'static str {
                self.0.name()
            }

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
