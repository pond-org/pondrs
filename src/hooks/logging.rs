//! Logging hook for pipeline execution.

#[cfg(feature = "std")]
use crate::core::PipelineItem;

#[cfg(feature = "std")]
use super::Hook;

#[cfg(feature = "std")]
pub struct LoggingHook;

#[cfg(feature = "std")]
impl Hook for LoggingHook {
    fn before_node_run(&self, n: &dyn PipelineItem) {
        let name = n.get_name();
        println!("Starting node {name}");
    }

    fn after_node_run(&self, n: &dyn PipelineItem) {
        let name = n.get_name();
        println!("Completed node {name}");
    }
}
