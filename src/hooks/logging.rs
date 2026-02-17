//! Logging hook for pipeline execution.

use crate::core::PipelineItem;

use super::Hook;

pub struct LoggingHook;

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
