//! Parallel pipeline runner.

use std::prelude::v1::*;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

use serde::Serialize;

use crate::core::Steps;
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

impl<H: Hooks + Sync> Runner for ParallelRunner<H> {
    fn run(&self, pipe: &impl Steps, catalog: &impl Serialize, params: &impl Serialize) {
        let graph = build_pipeline_graph(pipe, catalog, params);

        if graph.node_indices.is_empty() {
            return;
        }

        // Track whether each node has been started
        let started: Vec<AtomicBool> = graph.node_indices.iter().map(|_| AtomicBool::new(false)).collect();

        // Initialize produced with source datasets (params, pre-loaded data)
        let produced = Mutex::new(graph.source_datasets.clone());

        thread::scope(|s| {
            loop {
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
                        let item = node.item;

                        s.spawn(move || {
                            hooks.for_each_hook(&mut |h| h.before_node_run(item));
                            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                                item.call();
                            }));
                            match result {
                                Ok(()) => {
                                    hooks.for_each_hook(&mut |h| h.after_node_run(item));
                                    produced.lock().unwrap().extend(output_ids);
                                }
                                Err(e) => {
                                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                                        s.as_str()
                                    } else if let Some(s) = e.downcast_ref::<&str>() {
                                        s
                                    } else {
                                        "unknown panic"
                                    };
                                    hooks.for_each_hook(&mut |h| h.on_node_error(item, msg));
                                    panic::resume_unwind(e);
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
    }
}
