//! Parallel pipeline runner.

use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

use crate::core::{PipelineItem, Steps};
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

/// Collect all leaf nodes from the pipeline tree.
fn collect_leaves<'a>(pipe: &'a impl Steps) -> Vec<&'a dyn PipelineItem> {
    let mut leaves = Vec::new();
    pipe.for_each_item(&mut |item| {
        collect_item_leaves(item, &mut leaves);
    });
    leaves
}

fn collect_item_leaves<'a>(item: &'a dyn PipelineItem, leaves: &mut Vec<&'a dyn PipelineItem>) {
    if item.is_leaf() {
        leaves.push(item);
    } else {
        item.for_each_child(&mut |child| {
            collect_item_leaves(child, leaves);
        });
    }
}

impl<H: Hooks + Sync> Runner for ParallelRunner<H> {
    fn run(&self, pipe: &impl Steps) {
        let leaves = collect_leaves(pipe);

        if leaves.is_empty() {
            return;
        }

        // Track: node index -> (input_ids, output_ids, started)
        let nodes: Vec<_> = leaves
            .iter()
            .map(|n| {
                (
                    n.input_dataset_ids(),
                    n.output_dataset_ids(),
                    AtomicBool::new(false),
                )
            })
            .collect();

        // Find source datasets: inputs that are never produced by any node
        let all_outputs: HashSet<_> = nodes.iter().flat_map(|(_, outputs, _)| outputs.iter().copied()).collect();
        let all_inputs: HashSet<_> = nodes.iter().flat_map(|(inputs, _, _)| inputs.iter().copied()).collect();
        let sources: HashSet<_> = all_inputs.difference(&all_outputs).copied().collect();

        // Initialize produced with source datasets (Params, pre-loaded data)
        let produced = Mutex::new(sources);

        thread::scope(|s| {
            loop {
                let produced_snapshot: HashSet<_> = produced.lock().unwrap().clone();

                let mut any_started = false;
                for (i, node) in leaves.iter().enumerate() {
                    let (inputs, outputs, started) = &nodes[i];

                    if started.load(Ordering::Acquire) {
                        continue;
                    }

                    // Check if all inputs are produced
                    if inputs.iter().all(|id| produced_snapshot.contains(id)) {
                        started.store(true, Ordering::Release);
                        any_started = true;

                        let produced = &produced;
                        let outputs = outputs.clone();
                        let hooks = &self.hooks;

                        s.spawn(move || {
                            hooks.for_each_hook(&mut |h| h.before_node_run(*node));
                            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                                node.call();
                            }));
                            match result {
                                Ok(()) => {
                                    hooks.for_each_hook(&mut |h| h.after_node_run(*node));
                                    produced.lock().unwrap().extend(outputs);
                                }
                                Err(e) => {
                                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                                        s.as_str()
                                    } else if let Some(s) = e.downcast_ref::<&str>() {
                                        s
                                    } else {
                                        "unknown panic"
                                    };
                                    hooks.for_each_hook(&mut |h| h.on_node_error(*node, msg));
                                    panic::resume_unwind(e);
                                }
                            }
                        });
                    }
                }

                // Check if all nodes have started
                if nodes.iter().all(|(_, _, started)| started.load(Ordering::Acquire)) {
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
