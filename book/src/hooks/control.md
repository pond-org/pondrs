# HookControl & HookAbort

Hook methods return `Result` types that let hooks control pipeline execution. Before-hooks return `Result<HookControl, HookAbort>`, and after-hooks return `Result<(), HookAbort>`. Error hooks (`on_pipeline_error`, `on_node_error`) return `()` and cannot influence control flow.

## HookControl

```rust,ignore
#[non_exhaustive]
pub enum HookControl {
    Continue,
    Skip,
}
```

`Continue` proceeds normally. `Skip` tells the runner to skip the current operation.

When multiple hooks are composed in a tuple, their `HookControl` values are merged: if **any** hook returns `Skip`, the merged result is `Skip`. This uses `HookControl::merge()`, where Skip takes precedence over Continue.

### Where Skip takes effect

| Method | Skip behavior |
|--------|--------------|
| `before_node_run` | Node is skipped entirely — no datasets are loaded, no function executes, no datasets are saved. `after_node_run` fires with `skipped = true`. |
| `before_dataset_saved` | The save is skipped — the dataset is not written and `after_dataset_saved` does not fire. |
| `before_pipeline_run` | Reserved — not currently acted on by runners. |
| `before_dataset_loaded` | Reserved — not currently acted on. |

The primary use case for `Skip` is caching: `CacheHook` returns `Skip` from `before_node_run` when a node's inputs haven't changed since the last run. See [Built-in Hooks](./builtin.md) for details.

## HookAbort

```rust,ignore
pub struct HookAbort(pub &'static str);
```

Returning `Err(HookAbort("reason"))` from any hook method immediately stops the pipeline. The error propagates as `PondError::HookAbort("reason")`.

### Example: aborting on a condition

```rust,ignore
struct MaxNodesGuard {
    limit: usize,
    count: AtomicUsize,
}

impl Hook for MaxNodesGuard {
    fn before_node_run(&self, _n: &dyn StepInfo) -> Result<HookControl, HookAbort> {
        let c = self.count.fetch_add(1, Ordering::Relaxed);
        if c >= self.limit {
            Err(HookAbort("node limit exceeded"))
        } else {
            Ok(HookControl::Continue)
        }
    }
}
```

If this hook is active and the pipeline tries to run more nodes than `limit`, execution stops with `PondError::HookAbort("node limit exceeded")`.

### Example: skipping a node

```rust,ignore
struct SkipByName {
    skip: &'static str,
}

impl Hook for SkipByName {
    fn before_node_run(&self, n: &dyn StepInfo) -> Result<HookControl, HookAbort> {
        if n.name() == self.skip {
            Ok(HookControl::Skip)
        } else {
            Ok(HookControl::Continue)
        }
    }
}
```

When the runner encounters the named node, it skips it entirely and calls `after_node_run(n, skipped=true)`.
