//! Hooks for pipeline execution events.

mod logging;

pub use logging::LoggingHook;

use crate::core::PipelineItem;

/// Trait for individual hooks that respond to pipeline events.
pub trait Hook {
    fn before_node_run(&self, _n: &dyn PipelineItem) {}
    fn after_node_run(&self, _n: &dyn PipelineItem) {}
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
