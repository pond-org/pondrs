# Built-in Hooks

pondrs provides two built-in hook implementations.

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

`LoggingHook` uses a `TimingTracker` internally to measure durations between before/after pairs.

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
    VizHook::new("http://localhost:8080".into()),
    my_custom_hook,
))
```

Each hook receives every event independently. Order in the tuple determines call order, but hooks should not depend on ordering.
