//! Hooks for pipeline execution events.

#[cfg(feature = "std")]
mod logging;
#[cfg(feature = "std")]
pub(crate) mod timing;

#[cfg(feature = "std")]
pub use logging::LoggingHook;

#[cfg(feature = "std")]
mod cache;
#[cfg(feature = "std")]
pub use cache::CacheHook;

mod typed;
pub use typed::{TypedHook, TypedHookAdapter, IntoTypedHook};

use crate::pipeline::{DatasetRef, StepMeta};

#[derive(Debug, Clone)]
pub struct HookAbort(pub &'static str);

impl core::fmt::Display for HookAbort {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookControl {
    Continue,
    Skip,
}

impl Default for HookControl {
    fn default() -> Self {
        Self::Continue
    }
}

impl HookControl {
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Skip, _) | (_, Self::Skip) => Self::Skip,
            _ => Self::Continue,
        }
    }
}

/// Trait for individual hooks that respond to pipeline events.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a hook",
    label = "not a hook",
    note = "implement `Hook` for `{Self}`, or wrap a `TypedHook` with `.typed()`",
    note = "built-in hooks include `LoggingHook`, `CacheHook`, and `VizHook`"
)]
pub trait Hook: Sync {
    fn before_pipeline_run(&self, _p: &dyn StepMeta) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_pipeline_run(&self, _p: &dyn StepMeta) -> Result<(), HookAbort> { Ok(()) }
    fn on_pipeline_error(&self, _p: &dyn StepMeta, _error: &str) {}

    fn before_node_run(&self, _n: &dyn StepMeta) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_node_run(&self, _n: &dyn StepMeta, _skipped: bool) -> Result<(), HookAbort> { Ok(()) }
    fn on_node_error(&self, _n: &dyn StepMeta, _error: &str) {}

    fn before_dataset_loaded(&self, _n: &dyn StepMeta, _ds: &DatasetRef) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_dataset_loaded(&self, _n: &dyn StepMeta, _ds: &DatasetRef, _value: &dyn core::any::Any) -> Result<(), HookAbort> { Ok(()) }
    fn before_dataset_saved(&self, _n: &dyn StepMeta, _ds: &DatasetRef, _value: &dyn core::any::Any) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_dataset_saved(&self, _n: &dyn StepMeta, _ds: &DatasetRef) -> Result<(), HookAbort> { Ok(()) }
}

/// Trait for a collection of hooks (implemented for tuples).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a collection of hooks",
    label = "not a hooks tuple",
    note = "pass a tuple of types implementing `Hook`, e.g. `(LoggingHook,)`, or `()` for no hooks"
)]
pub trait Hooks: Sync {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook) -> Result<(), HookAbort>) -> Result<(), HookAbort>;
}

impl Hooks for () {
    fn for_each_hook(&self, _f: &mut dyn FnMut(&dyn Hook) -> Result<(), HookAbort>) -> Result<(), HookAbort> { Ok(()) }
}

macro_rules! impl_hooks {
    ($($H:ident $idx:tt),+) => {
        impl<$($H: Hook),+> Hooks for ($($H,)+) {
            fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook) -> Result<(), HookAbort>) -> Result<(), HookAbort> {
                $(f(&self.$idx)?;)+
                Ok(())
            }
        }
    };
}

impl_hooks!(H0 0);
impl_hooks!(H0 0, H1 1);
impl_hooks!(H0 0, H1 1, H2 2);
impl_hooks!(H0 0, H1 1, H2 2, H3 3);
impl_hooks!(H0 0, H1 1, H2 2, H3 3, H4 4);
impl_hooks!(H0 0, H1 1, H2 2, H3 3, H4 4, H5 5);
impl_hooks!(H0 0, H1 1, H2 2, H3 3, H4 4, H5 5, H6 6);
impl_hooks!(H0 0, H1 1, H2 2, H3 3, H4 4, H5 5, H6 6, H7 7);
impl_hooks!(H0 0, H1 1, H2 2, H3 3, H4 4, H5 5, H6 6, H7 7, H8 8);
impl_hooks!(H0 0, H1 1, H2 2, H3 3, H4 4, H5 5, H6 6, H7 7, H8 8, H9 9);
