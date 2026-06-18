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
- All input datasets must provide a **content hash** (`content_hash()` returns `Some`). File-backed datasets compute this from file metadata; `Param` datasets use their serialized value.

### Cache directory

Cache keys are stored as text files in the cache directory (default `.pondcache`). Each node gets one file named after a sanitized version of the node name. Delete the directory to force a full re-run.

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
