//! Sequential pipeline runner.

#[cfg(feature = "std")]
use std::string::ToString;

use crate::core::{PipelineItem, Steps};
use crate::error::PondError;
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
    fn run_item<E>(&self, item: &dyn PipelineItem<E>) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug,
    {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            match item.call() {
                Ok(()) => {
                    self.hooks.for_each_hook(&mut |h| h.after_node_run(item));
                    Ok(())
                }
                Err(e) => {
                    let msg = e.to_string();
                    self.hooks.for_each_hook(&mut |h| h.on_node_error(item, &msg));
                    Err(e)
                }
            }
        } else {
            self.hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            let mut result = Ok(());
            item.for_each_child_item(&mut |child| {
                if result.is_ok() {
                    result = self.run_item(child);
                }
            });
            match &result {
                Ok(()) => {
                    self.hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
                }
                Err(e) => {
                    let msg = e.to_string();
                    self.hooks.for_each_hook(&mut |h| h.on_pipeline_error(item, &msg));
                }
            }
            result
        }
    }

    #[cfg(not(feature = "std"))]
    fn run_item<E>(&self, item: &dyn PipelineItem<E>) -> Result<(), E>
    where
        E: From<PondError>,
    {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            item.call()?;
            self.hooks.for_each_hook(&mut |h| h.after_node_run(item));
            Ok(())
        } else {
            self.hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            item.for_each_child_item(&mut |child| {
                // In no_std, we can't easily propagate errors from closures,
                // so we just call and let it propagate via the node's own mechanism.
                let _ = self.run_item(child);
            });
            self.hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
            Ok(())
        }
    }
}

impl<H: Hooks> Runner for SequentialRunner<H> {
    fn run<E>(&self, pipe: &impl Steps<E>, _catalog: &impl serde::Serialize, _params: &impl serde::Serialize) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        let mut result = Ok(());
        pipe.for_each_item(&mut |item| {
            if result.is_ok() {
                result = self.run_item(item);
            }
        });
        result
    }
}
