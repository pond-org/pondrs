//! Parallel pipeline runner.

use std::prelude::v1::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

use serde::Serialize;

use crate::core::{PipelineItem, Steps};
use crate::error::PondError;
use crate::graph::build_pipeline_graph;
use crate::hooks::Hooks;

use super::Runner;

pub struct ParallelRunner<H: Hooks> {
    pub hooks: H,
}

impl<H: Hooks> ParallelRunner<H> {
    pub fn new(hooks: H) -> Self {
        Self { hooks }
    }
}

/// Collect callable items by walking the tree in the same order as graph building.
fn collect_items<'a, E>(items: &mut Vec<&'a dyn PipelineItem<E>>, item: &'a dyn PipelineItem<E>) {
    items.push(item);
    if !item.is_leaf() {
        item.for_each_child_item(&mut |child| {
            collect_items(items, child);
        });
    }
}

impl<H: Hooks + Sync> Runner for ParallelRunner<H> {
    fn run<E>(&self, pipe: &impl Steps<E>, catalog: &impl Serialize, params: &impl Serialize) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        // Build graph using PipelineInfo (non-generic) for dependency analysis
        let graph = build_pipeline_graph(pipe, catalog, params);

        if graph.node_indices.is_empty() {
            return Ok(());
        }

        // Collect callable items in the same tree-walk order as graph building
        let mut callable_items: Vec<&dyn PipelineItem<E>> = Vec::new();
        pipe.for_each_item(&mut |item| {
            collect_items(&mut callable_items, item);
        });

        // Track whether each node has been started
        let started: Vec<AtomicBool> = graph.node_indices.iter().map(|_| AtomicBool::new(false)).collect();

        // Initialize produced with source datasets (params, pre-loaded data)
        let produced = Mutex::new(graph.source_datasets.clone());

        // Track first error — stop scheduling new nodes on error, let in-flight drain
        let first_error: Mutex<Option<E>> = Mutex::new(None);
        let has_error = AtomicBool::new(false);

        thread::scope(|s| {
            loop {
                // If an error occurred, stop scheduling new nodes
                if has_error.load(Ordering::Acquire) {
                    break;
                }

                let produced_snapshot: HashSet<_> = produced.lock().unwrap().clone();

                let mut any_started = false;
                for (si, &node_idx) in graph.node_indices.iter().enumerate() {
                    if started[si].load(Ordering::Acquire) {
                        continue;
                    }

                    let node = &graph.nodes[node_idx];

                    // Check if all inputs are produced
                    if node.inputs.iter().all(|d| produced_snapshot.contains(&d.id)) {
                        started[si].store(true, Ordering::Release);
                        any_started = true;

                        let produced = &produced;
                        let output_ids: Vec<usize> = node.outputs.iter().map(|d| d.id).collect();
                        let hooks = &self.hooks;
                        let item = callable_items[node_idx];
                        let first_error = &first_error;
                        let has_error = &has_error;

                        s.spawn(move || {
                            hooks.for_each_hook(&mut |h| h.before_node_run(item));
                            match item.call() {
                                Ok(()) => {
                                    hooks.for_each_hook(&mut |h| h.after_node_run(item));
                                    produced.lock().unwrap().extend(output_ids);
                                }
                                Err(e) => {
                                    let msg = e.to_string();
                                    hooks.for_each_hook(&mut |h| h.on_node_error(item, &msg));
                                    let mut guard = first_error.lock().unwrap();
                                    if guard.is_none() {
                                        *guard = Some(e);
                                    }
                                    has_error.store(true, Ordering::Release);
                                }
                            }
                        });
                    }
                }

                // Check if all nodes have started
                if started.iter().all(|s| s.load(Ordering::Acquire)) {
                    break;
                }

                // If no progress was made, yield to let running threads complete
                if !any_started {
                    thread::yield_now();
                }
            }
        });

        // Return the first error if any occurred
        match first_error.into_inner().unwrap() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}
