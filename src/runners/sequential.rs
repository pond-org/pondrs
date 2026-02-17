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

    fn run_item(&self, item: &dyn PipelineItem) {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            item.call();
            self.hooks.for_each_hook(&mut |h| h.after_node_run(item));
        } else {
            item.for_each_child(&mut |child| {
                self.run_item(child);
            });
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
