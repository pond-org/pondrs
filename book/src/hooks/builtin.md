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

1. **`before_node_run`** computes the node's cache key. If the node's outputs are all persistent and the key matches the one stored by the last run that ran it, it returns `HookControl::Skip`. Otherwise it logs why the node runs (cache disabled, outputs not persistent, an input has no content hash, no stored key, key changed).
2. **`after_node_run`** stores the key if the node ran, and records it for the outputs, so that downstream nodes key on it — also when the node was skipped.

The key hashes:

- the node's path and function type;
- each input's type and catalog configuration, plus either the key of the node that produced it in this run or, for a source dataset, its content hash;
- each output's type and catalog configuration.

A downstream key follows from its producers' keys, not from the bytes they wrote. A node re-run with unchanged inputs produces the same key, so its consumers still skip.

### Requirements

- All output datasets must be **persistent** (`is_persistent() == true`) for caching to apply. Nodes with `MemoryDataset` outputs always re-run because their outputs don't survive across runs.
- All source input datasets must provide a **content hash** (`content_hash()` returns `Some`). File-backed datasets compute this from file metadata (canonical path, size and modification time — the contents are not read); `Param` datasets use their serialized value. A file rewritten with the same size and a preserved mtime (`rsync -t`, `cp -p`) is therefore not seen as changed.

### The cache trusts its records

A node whose key matches is skipped **without checking that its outputs still exist**. That is what lets a run resume after a crash even though `RetentionHook` has pruned old checkpoints: the finished epochs skip, and the first unfinished one reads the newest checkpoint, which retention kept.

If a node has to run and one of its inputs was produced by a node skipped in this run but is gone (its `content_hash()` is `None`), the run stops before that node:

```text
Hook aborted: `epochs/0/val` needs an input produced by skipped node `epochs/0/train`,
but it is missing (pruned?) [path: out/weights_0.txt]; delete its cache entry or rerun with PONDRS_CACHE=off
```

Deleted outputs and code changes are not detected. Invalidate by hand (below).

### Cache directory and invalidation

Keys are stored under the cache directory (default `.pondcache`), one directory per segment of the node's path, with one file per output configuration: `.pondcache/epochs/3/train/5f0c…`. Two catalogs that write a node's outputs to different places, such as a scratch catalog for testing, keep separate keys and don't invalidate each other. Two that write the same place share one key, so the second one's run makes the first one re-run.

To re-run nodes, delete their entries:

```bash
rm -r .pondcache/epochs/3          # every node under epochs/3
rm -r .pondcache/epochs/*/train    # every epoch's train node
rm -r .pondcache                   # everything
```

Note that a node re-run this way keeps its key, so its consumers still skip; delete theirs too if they should re-run.

To ignore the cache for one run, set `PONDRS_CACHE=off` (or `0`, `false`), or build the hook with `CacheHook::new(dir).force(true)` and wire that to your own flag. Either way every node runs, and the keys are still stored, so the next run can skip again. The variable is read when the hook is constructed.

### Caveats

- **Filtered runs.** With `--nodes` / `--from-nodes`, a node whose producer is filtered out keys on that input's content hash instead of the producer's key. Its key then differs from a full run's, so switching between filtered and full runs re-runs it once each time. That costs a re-run, never a wrong skip.
- **Relative paths.** A dataset's configuration is hashed as written, so the same relative output path under two working directories counts as the same place.
- **Hash stability.** Keys use `std`'s `DefaultHasher`, which is not guaranteed stable across Rust versions. A change there makes every node re-run once.

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
- **`CacheHook`.** It skips a node on its cache key without checking that the outputs still exist. Resuming after a crash works, because the newest checkpoints are the ones kept. A change that re-runs an epoch whose input was removed stops the run before that epoch, naming the node and the skipped producer (see [the cache trusts its records](#the-cache-trusts-its-records)).
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

`VizHook` never fails your pipeline: while the viz server is unreachable it holds its events, and replays them in order once the server answers, so a viz server started mid-run still shows the nodes that ran before it. It tracks:

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
