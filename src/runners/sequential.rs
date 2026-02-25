//! Sequential pipeline runner.

use std::panic;

use crate::core::{PipelineItem, Steps};
use crate::hooks::Hooks;

use super::Runner;

pub struct SequentialRunner<H: Hooks> {
    pub hooks: H,
}

impl<H: Hooks> SequentialRunner<H> {
    pub fn new(hooks: H) -> Self {
        Self { hooks }
    }

    fn run_item(&self, item: &dyn PipelineItem) {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                item.call();
            }));
            match result {
                Ok(()) => {
                    self.hooks.for_each_hook(&mut |h| h.after_node_run(item));
                }
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.as_str()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s
                    } else {
                        "unknown panic"
                    };
                    self.hooks.for_each_hook(&mut |h| h.on_node_error(item, msg));
                    panic::resume_unwind(e);
                }
            }
        } else {
            self.hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                item.for_each_child(&mut |child| {
                    self.run_item(child);
                });
            }));
            match result {
                Ok(()) => {
                    self.hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
                }
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.as_str()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s
                    } else {
                        "unknown panic"
                    };
                    self.hooks.for_each_hook(&mut |h| h.on_pipeline_error(item, msg));
                    panic::resume_unwind(e);
                }
            }
        }
    }
}

impl<H: Hooks> Runner for SequentialRunner<H> {
    fn run(&self, pipe: &impl Steps) {
        pipe.for_each_item(&mut |item| {
            self.run_item(item);
        });
    }
}
