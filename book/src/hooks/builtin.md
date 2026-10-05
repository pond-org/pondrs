# Built-in Hooks

pondrs provides three built-in hook implementations.

## `LoggingHook`

*Requires the `std` feature.*

Logs pipeline and node lifecycle events using the `log` crate, with automatic timing:

```rust,ignore
use pondrs::hooks::LoggingHook;

App::new(catalog, params)
    .with_hooks((LoggingHook::new(),))
    .execute(pipeline)?;
```

Output (with `env_logger` at `info` level):

```text
[INFO] [pipeline] processing - starting
[INFO] [node] clean - starting
[INFO] [node] clean - completed (12.3ms)
[INFO] [node] transform - starting
[INFO] [node] transform - completed (5.1ms)
[INFO] [pipeline] processing - completed (17.8ms)
```

At `debug` level, dataset load/save events are also logged:

```text
[DEBUG]   loading readings
[DEBUG]   loaded readings (8.2ms)
[DEBUG]   saving summary
[DEBUG]   saved summary (0.1ms)
```

When a node is skipped (e.g. by `CacheHook`), `LoggingHook` logs it:

```text
[INFO] [node] clean - skipped (cached)
```

`LoggingHook` uses a `TimingTracker` internally to measure durations between before/after pairs.

## `CacheHook`

*Requires the `std` feature.*

Automatically skips nodes whose inputs have not changed since the last run, using content hashing to detect changes.

```rust,ignore
use pondrs::CacheHook;

App::new(catalog, params)
    .with_hooks((CacheHook::new(".pondcache"),))
    .execute(pipeline)?;
```

### How it works

`CacheHook` implements `before_node_run` and `after_node_run`:

1. **`before_node_run`** computes a cache key from the node name, function type, and content hashes of all input datasets. If the key matches the stored key from the last run, it returns `HookControl::Skip`.
2. **`after_node_run`** writes the cache key to disk (if the node ran) and records output dataset keys for downstream nodes.

### Requirements

- All output datasets must be **persistent** (`is_persistent() == true`) for caching to apply. Nodes with `MemoryDataset` outputs always re-run because their outputs don't survive across runs.
- All input datasets must provide a **content hash** (`content_hash()` returns `Some`). File-backed datasets compute this from file metadata (canonical path, size and modification time — the contents are not read); `Param` datasets use their serialized value. A file rewritten with the same size and a preserved mtime (`rsync -t`, `cp -p`) is therefore not seen as changed.

### Cache directory

Cache keys are stored as text files in the cache directory (default `.pondcache`). Each node gets one file named after a sanitized version of the node name. Delete the directory to force a full re-run.

## `RetentionHook`

*Requires the `std` feature.*

Keeps only the most recent saves among datasets whose names match a pattern, deleting older ones as new ones are saved. It is built for checkpoints in an unrolled epoch chain (see [`RecurrentNode`](../pipelines/recurrent.md)), where every epoch writes its weights and N full checkpoints would fill a disk:

```rust,ignore
use pondrs::hooks::RetentionHook;

App::new(catalog, params)
    .with_hooks((
        CacheHook::new(".pondcache"),
        RetentionHook::new("catalog.epochs.*.weights", 3).keep_every(10),
    ))
    .execute(pipeline)?;
```

- The pattern is a full dataset name as hooks see it, rooted at `catalog.`; `*` matches exactly one dotted segment.
- `keep_last` (here 3) keeps the most recent matching saves, in save order.
- `keep_every(n)` additionally never removes a save whose first numeric `*` segment is a multiple of `n`, leaving resume points further back.

Datasets are removed through `Dataset::remover()`, an owned handle the hook takes at save time. File datasets delete their file (`PlotlyDataset` both of its files), `PartitionedDataset` deletes its entries, and `LazyDataset` / `CacheDataset` delegate to the dataset they wrap — `CacheDataset` also dropping its in-memory copy. A matching dataset without a remover is skipped with a warning, logged once per dataset. Removal failures are logged, not raised.

Size `keep_last` against:

- **Consumers.** A removed value is gone for every later reader. Under `ParallelRunner`, a per-epoch eval node can still be pending when the next epoch saves, so `keep_last` must cover it.
- **`CacheHook`.** It skips a node on its cache key without checking that the outputs still exist. Resuming after a crash works, because the newest checkpoints are the ones kept. A change that re-runs an epoch whose input was removed fails with that input's load error.
- **Earlier runs.** Only saves of the current run are queued, so up to `keep_last` checkpoints left by an earlier run are not removed by a later one.

## `VizHook`

*Requires the `viz` feature.*

Posts live execution events to a running viz server via HTTP. This enables the interactive visualization to show real-time node status during `app run`:

```rust,ignore
use pondrs::viz::VizHook;

App::new(catalog, params)
    .with_hooks((LoggingHook::new(), VizHook::new("http://localhost:8080".into())))
    .execute(pipeline)?;
```

The typical workflow is:

1. Start the viz server: `my_app viz --port 8080`
2. In another terminal, run the pipeline with `VizHook` attached

`VizHook` is **fire-and-forget** — it silently ignores HTTP errors, so a missing viz server won't crash your pipeline. It tracks:

- Node start/end/error events
- Dataset load/save durations

Each event is posted as a `VizEvent` to `POST /api/status`, which the viz server broadcasts to connected WebSocket clients.

## Combining hooks

Hooks compose as tuples:

```rust,ignore
.with_hooks((
    LoggingHook::new(),
    CacheHook::new(".pondcache"),
    VizHook::new("http://localhost:8080".into()),
    my_custom_hook,
))
```

Each hook receives every event independently. Order in the tuple determines call order, but hooks should not depend on ordering.
