//! Sequential pipeline runner.

#[cfg(feature = "std")]
use std::prelude::v1::*;
#[cfg(feature = "std")]
use std::collections::HashMap;

use crate::pipeline::{DatasetEvent, DatasetRef, RunnableStep, Steps};
use crate::error::PondError;
use crate::hooks::Hooks;

use super::Runner;

pub struct SequentialRunner;

impl SequentialRunner {
    fn make_dataset_callback<'a, E>(
        item: &'a dyn RunnableStep<E>,
        #[cfg(feature = "std")]
        names: &'a HashMap<usize, String>,
        hooks: &'a impl Hooks,
    ) -> impl FnMut(&DatasetRef, DatasetEvent) + 'a {
        move |ds: &DatasetRef<'_>, event: DatasetEvent| {
            #[cfg(feature = "std")]
            super::dispatch_dataset_event(item, ds, event, names, hooks);
            #[cfg(not(feature = "std"))]
            super::dispatch_dataset_event_raw(item, ds, event, hooks);
        }
    }

    fn run_item<E>(
        &self,
        item: &dyn RunnableStep<E>,
        #[cfg(feature = "std")]
        names: &HashMap<usize, String>,
        hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug,
    {
        if item.is_leaf() {
            hooks.for_each_hook(&mut |h| h.before_node_run(item));
            #[cfg(feature = "std")]
            let mut on_event = Self::make_dataset_callback(item, names, hooks);
            #[cfg(not(feature = "std"))]
            let mut on_event = Self::make_dataset_callback(item, hooks);
            match item.call(&mut on_event) {
                Ok(()) => {
                    hooks.for_each_hook(&mut |h| h.after_node_run(item));
                    Ok(())
                }
                Err(e) => {
                    #[cfg(feature = "std")]
                    let msg = e.to_string();
                    #[cfg(not(feature = "std"))]
                    let msg = "node error";
                    hooks.for_each_hook(&mut |h| h.on_node_error(item, &msg));
                    Err(e)
                }
            }
        } else {
            hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            let mut result = Ok(());
            item.for_each_child_step(&mut |child| {
                if result.is_ok() {
                    #[cfg(feature = "std")]
                    { result = self.run_item(child, names, hooks); }
                    #[cfg(not(feature = "std"))]
                    { result = self.run_item(child, hooks); }
                }
            });
            match &result {
                Ok(()) => {
                    hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
                }
                Err(_e) => {
                    #[cfg(feature = "std")]
                    let msg = _e.to_string();
                    #[cfg(not(feature = "std"))]
                    let msg = "pipeline error";
                    hooks.for_each_hook(&mut |h| h.on_pipeline_error(item, &msg));
                }
            }
            result
        }
    }
}

impl Runner for SequentialRunner {
    fn name(&self) -> &'static str {
        "sequential"
    }

    fn run<E>(&self, pipe: &impl Steps<E>, catalog: &impl serde::Serialize, params: &impl serde::Serialize, hooks: &impl Hooks) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        #[cfg(feature = "std")]
        let names = crate::catalog_indexer::index_catalog_with_params(catalog, params).into_inner();
        #[cfg(not(feature = "std"))]
        let _ = (catalog, params);

        let mut result = Ok(());
        pipe.for_each_item(&mut |item| {
            if result.is_ok() {
                #[cfg(feature = "std")]
                { result = self.run_item(item, &names, hooks); }
                #[cfg(not(feature = "std"))]
                { result = self.run_item(item, hooks); }
            }
        });
        result
    }
}
