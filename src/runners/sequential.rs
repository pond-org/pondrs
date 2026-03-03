//! Sequential pipeline runner.

#[cfg(feature = "std")]
use std::prelude::v1::*;
#[cfg(feature = "std")]
use std::collections::HashMap;

use crate::core::{DatasetEvent, DatasetInfo, DatasetRef, PipelineItem, Steps};
use crate::error::PondError;
use crate::hooks::Hooks;

use super::Runner;

pub struct SequentialRunner;

impl SequentialRunner {
    #[cfg(feature = "std")]
    fn make_dataset_callback<'a, E>(
        item: &'a dyn PipelineItem<E>,
        names: &'a HashMap<usize, String>,
        hooks: &'a impl Hooks,
    ) -> impl FnMut(&DatasetRef, DatasetEvent) + 'a {
        move |ds: &DatasetRef<'_>, event: DatasetEvent| {
            let info = DatasetInfo {
                id: ds.id,
                is_param: ds.meta.is_param(),
                name: names.get(&ds.id).map(|s| s.as_str()),
            };
            match event {
                DatasetEvent::BeforeLoad => hooks.for_each_hook(&mut |h| h.before_dataset_load(item, &info)),
                DatasetEvent::AfterLoad => hooks.for_each_hook(&mut |h| h.after_dataset_load(item, &info)),
                DatasetEvent::BeforeSave => hooks.for_each_hook(&mut |h| h.before_dataset_save(item, &info)),
                DatasetEvent::AfterSave => hooks.for_each_hook(&mut |h| h.after_dataset_save(item, &info)),
            }
        }
    }

    #[cfg(not(feature = "std"))]
    fn make_dataset_callback<'a, E>(
        item: &'a dyn PipelineItem<E>,
        hooks: &'a impl Hooks,
    ) -> impl FnMut(&DatasetRef, DatasetEvent) + 'a {
        move |ds: &DatasetRef<'_>, event: DatasetEvent| {
            let info = DatasetInfo {
                id: ds.id,
                is_param: ds.meta.is_param(),
                name: None,
            };
            match event {
                DatasetEvent::BeforeLoad => hooks.for_each_hook(&mut |h| h.before_dataset_load(item, &info)),
                DatasetEvent::AfterLoad => hooks.for_each_hook(&mut |h| h.after_dataset_load(item, &info)),
                DatasetEvent::BeforeSave => hooks.for_each_hook(&mut |h| h.before_dataset_save(item, &info)),
                DatasetEvent::AfterSave => hooks.for_each_hook(&mut |h| h.after_dataset_save(item, &info)),
            }
        }
    }

    #[cfg(feature = "std")]
    fn run_item<E>(&self, item: &dyn PipelineItem<E>, names: &HashMap<usize, String>, hooks: &impl Hooks) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug,
    {
        if item.is_leaf() {
            hooks.for_each_hook(&mut |h| h.before_node_run(item));
            let mut on_event = Self::make_dataset_callback(item, names, hooks);
            match item.call(&mut on_event) {
                Ok(()) => {
                    hooks.for_each_hook(&mut |h| h.after_node_run(item));
                    Ok(())
                }
                Err(e) => {
                    let msg = e.to_string();
                    hooks.for_each_hook(&mut |h| h.on_node_error(item, &msg));
                    Err(e)
                }
            }
        } else {
            hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            let mut result = Ok(());
            item.for_each_child_item(&mut |child| {
                if result.is_ok() {
                    result = self.run_item(child, names, hooks);
                }
            });
            match &result {
                Ok(()) => {
                    hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
                }
                Err(e) => {
                    let msg = e.to_string();
                    hooks.for_each_hook(&mut |h| h.on_pipeline_error(item, &msg));
                }
            }
            result
        }
    }

    #[cfg(not(feature = "std"))]
    fn run_item<E>(&self, item: &dyn PipelineItem<E>, hooks: &impl Hooks) -> Result<(), E>
    where
        E: From<PondError>,
    {
        if item.is_leaf() {
            hooks.for_each_hook(&mut |h| h.before_node_run(item));
            let mut on_event = Self::make_dataset_callback(item, hooks);
            item.call(&mut on_event)?;
            hooks.for_each_hook(&mut |h| h.after_node_run(item));
            Ok(())
        } else {
            hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            item.for_each_child_item(&mut |child| {
                let _ = self.run_item(child, hooks);
            });
            hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
            Ok(())
        }
    }
}

impl Runner for SequentialRunner {
    fn name(&self) -> &'static str {
        "sequential"
    }

    #[cfg(feature = "std")]
    fn run<E>(&self, pipe: &impl Steps<E>, catalog: &impl serde::Serialize, params: &impl serde::Serialize, hooks: &impl Hooks) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        let names = crate::catalog_indexer::index_catalog_with_params(catalog, params).into_inner();
        let mut result = Ok(());
        pipe.for_each_item(&mut |item| {
            if result.is_ok() {
                result = self.run_item(item, &names, hooks);
            }
        });
        result
    }

    #[cfg(not(feature = "std"))]
    fn run<E>(&self, pipe: &impl Steps<E>, _catalog: &impl serde::Serialize, _params: &impl serde::Serialize, hooks: &impl Hooks) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        let mut result = Ok(());
        pipe.for_each_item(&mut |item| {
            if result.is_ok() {
                result = self.run_item(item, hooks);
            }
        });
        result
    }
}
