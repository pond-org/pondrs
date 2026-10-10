//! Sequential pipeline runner.

#[cfg(feature = "std")]
use std::prelude::v1::*;
#[cfg(feature = "std")]
use std::collections::HashMap;

use crate::pipeline::{DatasetEvent, DatasetRef, Step, StepKind, StepMeta, Steps};
#[cfg(feature = "std")]
use crate::pipeline::path::{self, Qualified};
use crate::error::PondError;
use crate::hooks::{HookAbort, HookControl, Hooks};

use super::Runner;

/// Runs pipeline steps one at a time in definition order.
///
/// Available in both `no_std` and `std` environments.
#[derive(Default)]
pub struct SequentialRunner;

impl SequentialRunner {
    fn make_dataset_callback<'a>(
        item: &'a dyn StepMeta,
        #[cfg(feature = "std")]
        names: &'a HashMap<usize, String>,
        hooks: &'a impl Hooks,
    ) -> impl FnMut(&DatasetRef, DatasetEvent<'_>) -> Result<HookControl, HookAbort> + 'a {
        move |ds: &DatasetRef<'_>, event: DatasetEvent<'_>| {
            #[cfg(feature = "std")]
            { super::dispatch_dataset_event(item, ds, event, names, hooks) }
            #[cfg(not(feature = "std"))]
            { super::dispatch_dataset_event_raw(item, ds, event, hooks) }
        }
    }

    /// Run one step. Under `std`, `parent` is the enclosing group's path, and
    /// hooks see the step under its own full path; without an allocator there
    /// is no path to build, and hooks see the local name.
    fn run_item<E>(
        item: &dyn Step<E>,
        #[cfg(feature = "std")]
        parent: Option<&str>,
        #[cfg(feature = "std")]
        names: &HashMap<usize, String>,
        hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug,
    {
        #[cfg(feature = "std")]
        let qualified = Qualified::new(item, path::join(parent, item.name()));
        #[cfg(feature = "std")]
        let meta: &dyn StepMeta = &qualified;
        #[cfg(not(feature = "std"))]
        let meta: &dyn StepMeta = item;

        match item.kind() {
            StepKind::Leaf(leaf) => {
                let control = super::fire_before_node(hooks, meta)?;
                if control == HookControl::Skip {
                    super::fire_after_node(hooks, meta, true)?;
                    return Ok(());
                }
                #[cfg(feature = "std")]
                let mut on_event = Self::make_dataset_callback(meta, names, hooks);
                #[cfg(not(feature = "std"))]
                let mut on_event = Self::make_dataset_callback(meta, hooks);
                match leaf.call(&mut on_event) {
                    Ok(()) => {
                        super::fire_after_node(hooks, meta, false)?;
                        Ok(())
                    }
                    Err(e) => {
                        #[cfg(feature = "std")]
                        super::fire_node_error(hooks, meta, &e.to_string());
                        // Without an allocator there is nothing to render `e` into.
                        #[cfg(not(feature = "std"))]
                        super::fire_node_error(hooks, meta, "node error");
                        Err(e)
                    }
                }
            }
            StepKind::Group(group) => {
                super::fire_before_pipeline(hooks, meta)?;
                let mut result = Ok(());
                group.for_each_child_step(&mut |child| {
                    if result.is_ok() {
                        #[cfg(feature = "std")]
                        { result = Self::run_item(child, Some(meta.name()), names, hooks); }
                        #[cfg(not(feature = "std"))]
                        { result = Self::run_item(child, hooks); }
                    }
                });
                match &result {
                    Ok(()) => {
                        super::fire_after_pipeline(hooks, meta)?;
                    }
                    Err(e) => {
                        #[cfg(feature = "std")]
                        super::fire_pipeline_error(hooks, meta, &e.to_string());
                        // Without an allocator there is nothing to render `e` into.
                        #[cfg(not(feature = "std"))]
                        {
                            let _ = e;
                            super::fire_pipeline_error(hooks, meta, "pipeline error");
                        }
                    }
                }
                result
            }
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
        pipe.for_each_step(&mut |item| {
            if result.is_ok() {
                #[cfg(feature = "std")]
                { result = Self::run_item(item, None, &names, hooks); }
                #[cfg(not(feature = "std"))]
                { result = Self::run_item(item, hooks); }
            }
        });
        result
    }
}
