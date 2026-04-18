//! Hooks for pipeline execution events.

#[cfg(feature = "std")]
mod logging;
#[cfg(feature = "std")]
pub(crate) mod timing;

#[cfg(feature = "std")]
pub use logging::LoggingHook;

use crate::pipeline::{DatasetRef, StepInfo};

/// Trait for individual hooks that respond to pipeline events.
pub trait Hook: Sync {
    // Pipeline hooks
    fn before_pipeline_run(&self, _p: &dyn StepInfo) {}
    fn after_pipeline_run(&self, _p: &dyn StepInfo) {}
    fn on_pipeline_error(&self, _p: &dyn StepInfo, _error: &str) {}

    // Node hooks
    fn before_node_run(&self, _n: &dyn StepInfo) {}
    fn after_node_run(&self, _n: &dyn StepInfo) {}
    fn on_node_error(&self, _n: &dyn StepInfo, _error: &str) {}

    // Dataset hooks — fired per-dataset during Node::call()
    fn before_dataset_loaded(&self, _n: &dyn StepInfo, _ds: &DatasetRef) {}
    fn after_dataset_loaded(&self, _n: &dyn StepInfo, _ds: &DatasetRef) {}
    fn before_dataset_saved(&self, _n: &dyn StepInfo, _ds: &DatasetRef) {}
    fn after_dataset_saved(&self, _n: &dyn StepInfo, _ds: &DatasetRef) {}
}

/// Trait for a collection of hooks (implemented for tuples).
pub trait Hooks: Sync {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook));
}

impl Hooks for () {
    fn for_each_hook(&self, _f: &mut dyn FnMut(&dyn Hook)) {}
}

macro_rules! impl_hooks {
    ($($H:ident $idx:tt),+) => {
        impl<$($H: Hook),+> Hooks for ($($H,)+) {
            fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook)) {
                $(f(&self.$idx);)+
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
