# Node Hooks

Node hooks fire when a runner starts and finishes executing a node, or when a node encounters an error.

## Methods

```rust,ignore
fn before_node_run(&self, n: &dyn PipelineInfo) {}
fn after_node_run(&self, n: &dyn PipelineInfo) {}
fn on_node_error(&self, n: &dyn PipelineInfo, error: &str) {}
```

## Arguments

- **`n`** — the node being executed. Use `n.name()` for the node name, `n.type_string()` for the function's type name.
- **`error`** (on `on_node_error`) — the stringified error message. In `std` builds this is `e.to_string()`; in `no_std` it's the fixed string `"node error"`.

## Lifecycle

For a successful node execution:

```text
before_node_run(n)
  before_dataset_loaded(n, ds0)
  after_dataset_loaded(n, ds0)
  ... (function executes) ...
  before_dataset_saved(n, ds_out)
  after_dataset_saved(n, ds_out)
after_node_run(n)
```

For a failed node:

```text
before_node_run(n)
  ... (error occurs during load, function, or save) ...
on_node_error(n, "error message")
```

`after_node_run` and `on_node_error` are mutually exclusive — exactly one fires per node execution.

## Example: counting nodes

```rust,ignore
use std::sync::atomic::{AtomicUsize, Ordering};

struct NodeCounter {
    count: AtomicUsize,
}

impl Hook for NodeCounter {
    fn after_node_run(&self, n: &dyn PipelineInfo) {
        let i = self.count.fetch_add(1, Ordering::Relaxed) + 1;
        println!("Completed node {} ({}/total)", n.name(), i);
    }

    fn on_node_error(&self, n: &dyn PipelineInfo, error: &str) {
        eprintln!("Node {} failed: {}", n.name(), error);
    }
}
```

## Parallel runner behavior

With the `ParallelRunner`, node hooks may fire from different threads concurrently. This is why `Hook: Sync` is required. Use atomic types or `Mutex` for any shared state in your hook.
