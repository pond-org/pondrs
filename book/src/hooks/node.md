# Node Hooks

Node hooks fire when a runner starts and finishes executing a node, or when a node encounters an error.

## Methods

```rust,ignore
fn before_node_run(&self, n: &dyn StepInfo) -> Result<HookControl, HookAbort> { Ok(HookControl::Continue) }
fn after_node_run(&self, n: &dyn StepInfo, skipped: bool) -> Result<(), HookAbort> { Ok(()) }
fn on_node_error(&self, n: &dyn StepInfo, error: &str) {}
```

## Arguments

- **`n`** — the node being executed. Use `n.name()` for the node name, `n.type_string()` for the function's type name.
- **`skipped`** (on `after_node_run`) — `true` if the node was skipped because a `before_node_run` hook returned `HookControl::Skip`. When skipped, no datasets are loaded, the node function does not execute, and no datasets are saved.
- **`error`** (on `on_node_error`) — the stringified error message. In `std` builds this is `e.to_string()`; in `no_std` it's the fixed string `"node error"`.

## Lifecycle

For a successful node execution:

```text
before_node_run(n) -> Ok(Continue)
  before_dataset_loaded(n, ds0)
  after_dataset_loaded(n, ds0, &value)
  ... (function executes) ...
  before_dataset_saved(n, ds_out, &output)
  after_dataset_saved(n, ds_out)
after_node_run(n, skipped=false)
```

For a skipped node (hook returned `Skip`):

```text
before_node_run(n) -> Ok(Skip)
after_node_run(n, skipped=true)
```

For a failed node:

```text
before_node_run(n) -> Ok(Continue)
  ... (error occurs during load, function, or save) ...
on_node_error(n, "error message")
```

For an aborted node (hook returned `Err`):

```text
before_node_run(n) -> Err(HookAbort("reason"))
  pipeline stops with PondError::HookAbort
```

`after_node_run` and `on_node_error` are mutually exclusive — exactly one fires per successful or failed node execution. Neither fires when a hook aborts the pipeline from `before_node_run`.

## Example: counting nodes

```rust,ignore
use std::sync::atomic::{AtomicUsize, Ordering};

struct NodeCounter {
    count: AtomicUsize,
}

impl Hook for NodeCounter {
    fn after_node_run(&self, n: &dyn StepInfo, skipped: bool) -> Result<(), HookAbort> {
        if !skipped {
            let i = self.count.fetch_add(1, Ordering::Relaxed) + 1;
            println!("Completed node {} ({}/total)", n.name(), i);
        }
        Ok(())
    }

    fn on_node_error(&self, n: &dyn StepInfo, error: &str) {
        eprintln!("Node {} failed: {}", n.name(), error);
    }
}
```

## Parallel runner behavior

With the `ParallelRunner`, node hooks may fire from different threads concurrently. This is why `Hook: Sync` is required. Use atomic types or `Mutex` for any shared state in your hook.
