//! Hooks for pipeline execution events.

#[cfg(feature = "std")]
mod logging;

#[cfg(feature = "std")]
pub use logging::LoggingHook;

use crate::core::PipelineInfo;

/// Trait for individual hooks that respond to pipeline events.
pub trait Hook {
    // Pipeline hooks
    fn before_pipeline_run(&self, _p: &dyn PipelineInfo) {}
    fn after_pipeline_run(&self, _p: &dyn PipelineInfo) {}
    fn on_pipeline_error(&self, _p: &dyn PipelineInfo, _error: &str) {}

    // Node hooks
    fn before_node_run(&self, _n: &dyn PipelineInfo) {}
    fn after_node_run(&self, _n: &dyn PipelineInfo) {}
    fn on_node_error(&self, _n: &dyn PipelineInfo, _error: &str) {}

    // Dataset hooks (wiring TBD — requires splitting PipelineItem::call())
    fn before_dataset_load(&self, _n: &dyn PipelineInfo) {}
    fn after_dataset_load(&self, _n: &dyn PipelineInfo) {}
    fn before_dataset_save(&self, _n: &dyn PipelineInfo) {}
    fn after_dataset_save(&self, _n: &dyn PipelineInfo) {}
}

/// Trait for a collection of hooks (implemented for tuples).
pub trait Hooks {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook));
}

impl Hooks for () {
    fn for_each_hook(&self, _f: &mut dyn FnMut(&dyn Hook)) {}
}

impl<H: Hook> Hooks for (H,) {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook)) {
        f(&self.0);
    }
}

impl<H1: Hook, H2: Hook> Hooks for (H1, H2) {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook)) {
        f(&self.0);
        f(&self.1);
    }
}

impl<H1: Hook, H2: Hook, H3: Hook> Hooks for (H1, H2, H3) {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
    }
}
