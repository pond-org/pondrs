//! Parallel pipeline runner.

use std::prelude::v1::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

use serde::Serialize;

use crate::pipeline::{DatasetEvent, DatasetRef, RunnableStep, Steps};
use crate::error::PondError;
use crate::graph::build_pipeline_graph;
use crate::hooks::Hooks;

use super::Runner;

#[derive(Default)]
pub struct ParallelRunner;

/// Collect callable items by walking the tree in the same order as graph building.
fn collect_items<'a, E>(items: &mut Vec<&'a dyn RunnableStep<E>>, item: &'a dyn RunnableStep<E>) {
    items.push(item);
    if !item.is_leaf() {
        item.for_each_child_step(&mut |child| {
            collect_items(items, child);
        });
    }
}

impl Runner for ParallelRunner {
    fn name(&self) -> &'static str {
        "parallel"
    }

    fn run<E>(&self, pipe: &impl Steps<E>, catalog: &impl Serialize, params: &impl Serialize, hooks: &impl Hooks) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static,
    {
        // Build graph using PipelineInfo (non-generic) for dependency analysis
        let graph = build_pipeline_graph(pipe, catalog, params);

        if graph.node_indices.is_empty() {
            return Ok(());
        }

        // Collect callable items in the same tree-walk order as graph building
        let mut callable_items: Vec<&dyn RunnableStep<E>> = Vec::new();
        pipe.for_each_item(&mut |item| {
            collect_items(&mut callable_items, item);
        });

        // Track whether each leaf node has been started
        let started: Vec<AtomicBool> = graph.node_indices.iter().map(|_| AtomicBool::new(false)).collect();

        // Track pipeline lifecycle: indices into graph.nodes for pipe nodes
        let pipe_indices: Vec<usize> = graph.nodes.iter().enumerate()
            .filter(|(_, n)| n.is_pipe)
            .map(|(i, _)| i)
            .collect();
        let pipe_started: Vec<AtomicBool> = pipe_indices.iter().map(|_| AtomicBool::new(false)).collect();
        let pipe_completed: Vec<AtomicBool> = pipe_indices.iter().map(|_| AtomicBool::new(false)).collect();

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

                // Fire pipeline hooks based on produced datasets
                for (pi, &pipe_idx) in pipe_indices.iter().enumerate() {
                    let pipe_node = &graph.nodes[pipe_idx];

                    // before_pipeline_run: all inputs produced
                    if !pipe_started[pi].load(Ordering::Acquire)
                        && pipe_node.inputs.iter().all(|d| produced_snapshot.contains(&d.id))
                    {
                        pipe_started[pi].store(true, Ordering::Release);
                        hooks.for_each_hook(&mut |h| h.before_pipeline_run(pipe_node.item));
                    }

                    // after_pipeline_run: all outputs produced
                    if pipe_started[pi].load(Ordering::Acquire)
                        && !pipe_completed[pi].load(Ordering::Acquire)
                        && pipe_node.outputs.iter().all(|d| produced_snapshot.contains(&d.id))
                    {
                        pipe_completed[pi].store(true, Ordering::Release);
                        hooks.for_each_hook(&mut |h| h.after_pipeline_run(pipe_node.item));
                    }
                }

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
                        let item = callable_items[node_idx];
                        let first_error = &first_error;
                        let has_error = &has_error;
                        let graph_nodes = &graph.nodes;

                        let names = &graph.dataset_names;
                        s.spawn(move || {
                            hooks.for_each_hook(&mut |h| h.before_node_run(item));
                            let mut on_event = |ds: &DatasetRef<'_>, event: DatasetEvent| {
                                super::dispatch_dataset_event(item, ds, event, names, hooks);
                            };
                            match item.call(&mut on_event) {
                                Ok(()) => {
                                    hooks.for_each_hook(&mut |h| h.after_node_run(item));
                                    produced.lock().unwrap().extend(output_ids);
                                }
                                Err(e) => {
                                    let msg = e.to_string();
                                    hooks.for_each_hook(&mut |h| h.on_node_error(item, &msg));
                                    // Fire on_pipeline_error for ancestor pipelines
                                    let mut parent = graph_nodes[node_idx].parent_pipe;
                                    while let Some(pipe_idx) = parent {
                                        let pipe = &graph_nodes[pipe_idx];
                                        hooks.for_each_hook(&mut |h| h.on_pipeline_error(pipe.item, &msg));
                                        parent = pipe.parent_pipe;
                                    }
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

        // Fire any remaining pipeline completions after all threads have joined
        {
            let produced_snapshot = produced.lock().unwrap();
            for (pi, &pipe_idx) in pipe_indices.iter().enumerate() {
                let pipe_node = &graph.nodes[pipe_idx];
                if pipe_started[pi].load(Ordering::Acquire)
                    && !pipe_completed[pi].load(Ordering::Acquire)
                    && pipe_node.outputs.iter().all(|d| produced_snapshot.contains(&d.id))
                {
                    hooks.for_each_hook(&mut |h| h.after_pipeline_run(pipe_node.item));
                }
            }
        }

        // Return the first error if any occurred
        match first_error.into_inner().unwrap() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}
