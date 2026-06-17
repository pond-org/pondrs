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

use crate::pipeline::{DatasetRef, StepInfo};

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
        HookControl::Continue
    }
}

impl HookControl {
    pub fn merge(self, other: HookControl) -> HookControl {
        match (self, other) {
            (HookControl::Skip, _) | (_, HookControl::Skip) => HookControl::Skip,
            _ => HookControl::Continue,
        }
    }
}

/// Trait for individual hooks that respond to pipeline events.
pub trait Hook: Sync {
    fn before_pipeline_run(&self, _p: &dyn StepInfo) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_pipeline_run(&self, _p: &dyn StepInfo) -> Result<(), HookAbort> { Ok(()) }
    fn on_pipeline_error(&self, _p: &dyn StepInfo, _error: &str) {}

    fn before_node_run(&self, _n: &dyn StepInfo) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_node_run(&self, _n: &dyn StepInfo, _skipped: bool) -> Result<(), HookAbort> { Ok(()) }
    fn on_node_error(&self, _n: &dyn StepInfo, _error: &str) {}

    fn before_dataset_loaded(&self, _n: &dyn StepInfo, _ds: &DatasetRef) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_dataset_loaded(&self, _n: &dyn StepInfo, _ds: &DatasetRef, _value: &dyn core::any::Any) -> Result<(), HookAbort> { Ok(()) }
    fn before_dataset_saved(&self, _n: &dyn StepInfo, _ds: &DatasetRef, _value: &dyn core::any::Any) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
    fn after_dataset_saved(&self, _n: &dyn StepInfo, _ds: &DatasetRef) -> Result<(), HookAbort> { Ok(()) }
}

/// Trait for a collection of hooks (implemented for tuples).
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
