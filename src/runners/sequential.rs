//! Sequential pipeline runner.

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

    #[cfg(feature = "std")]
    fn run_item(&self, item: &dyn PipelineItem) {
        use std::prelude::v1::*;

        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                    std::panic::resume_unwind(e);
                }
            }
        } else {
            self.hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                    std::panic::resume_unwind(e);
                }
            }
        }
    }

    #[cfg(not(feature = "std"))]
    fn run_item(&self, item: &dyn PipelineItem) {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            item.call();
            self.hooks.for_each_hook(&mut |h| h.after_node_run(item));
        } else {
            self.hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            item.for_each_child(&mut |child| {
                self.run_item(child);
            });
            self.hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
        }
    }
}

impl<H: Hooks> Runner for SequentialRunner<H> {
    fn run(&self, pipe: &impl Steps, _catalog: &impl serde::Serialize, _params: &impl serde::Serialize) {
        pipe.for_each_item(&mut |item| {
            self.run_item(item);
        });
    }
}
