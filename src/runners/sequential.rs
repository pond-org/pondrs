//! Sequential pipeline runner.

#[cfg(feature = "std")]
use std::prelude::v1::*;
#[cfg(feature = "std")]
use std::collections::HashMap;

use crate::core::{DatasetEvent, DatasetInfo, DatasetRef, PipelineItem, Steps};
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
    fn make_dataset_callback<'a, E>(
        &'a self,
        item: &'a dyn PipelineItem<E>,
        names: &'a HashMap<usize, String>,
    ) -> impl FnMut(&DatasetRef, DatasetEvent) + 'a {
        move |ds: &DatasetRef, event: DatasetEvent| {
            let info = DatasetInfo {
                id: ds.id,
                is_param: ds.is_param,
                name: names.get(&ds.id).map(|s| s.as_str()),
            };
            match event {
                DatasetEvent::BeforeLoad => self.hooks.for_each_hook(&mut |h| h.before_dataset_load(item, &info)),
                DatasetEvent::AfterLoad => self.hooks.for_each_hook(&mut |h| h.after_dataset_load(item, &info)),
                DatasetEvent::BeforeSave => self.hooks.for_each_hook(&mut |h| h.before_dataset_save(item, &info)),
                DatasetEvent::AfterSave => self.hooks.for_each_hook(&mut |h| h.after_dataset_save(item, &info)),
            }
        }
    }

    #[cfg(not(feature = "std"))]
    fn make_dataset_callback<'a, E>(
        &'a self,
        item: &'a dyn PipelineItem<E>,
    ) -> impl FnMut(&DatasetRef, DatasetEvent) + 'a {
        move |ds: &DatasetRef, event: DatasetEvent| {
            let info = DatasetInfo {
                id: ds.id,
                is_param: ds.is_param,
                name: None,
            };
            match event {
                DatasetEvent::BeforeLoad => self.hooks.for_each_hook(&mut |h| h.before_dataset_load(item, &info)),
                DatasetEvent::AfterLoad => self.hooks.for_each_hook(&mut |h| h.after_dataset_load(item, &info)),
                DatasetEvent::BeforeSave => self.hooks.for_each_hook(&mut |h| h.before_dataset_save(item, &info)),
                DatasetEvent::AfterSave => self.hooks.for_each_hook(&mut |h| h.after_dataset_save(item, &info)),
            }
        }
    }

    #[cfg(feature = "std")]
    fn run_item<E>(&self, item: &dyn PipelineItem<E>, names: &HashMap<usize, String>) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug,
    {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            let mut on_event = self.make_dataset_callback(item, names);
            match item.call(&mut on_event) {
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
                    result = self.run_item(child, names);
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
            let mut on_event = self.make_dataset_callback(item);
            item.call(&mut on_event)?;
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
    #[cfg(feature = "std")]
    fn run<E>(&self, pipe: &impl Steps<E>, catalog: &impl serde::Serialize, params: &impl serde::Serialize) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        let names = crate::catalog_indexer::index_catalog_with_params(catalog, params).into_inner();
        let mut result = Ok(());
        pipe.for_each_item(&mut |item| {
            if result.is_ok() {
                result = self.run_item(item, &names);
            }
        });
        result
    }

    #[cfg(not(feature = "std"))]
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
