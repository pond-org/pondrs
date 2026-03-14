# App & Debugging in no_std

## Compiling a no_std pipeline with std

One of the strengths of pondrs is that a pipeline written for `no_std` can also be compiled with `std` enabled. This lets you use the full `App` interface — including YAML configuration, CLI argument parsing, parameter overrides, `check`, and `viz` — for development and debugging, while deploying the same pipeline code to a `no_std` target.

Simply add `pondrs` as a dev-dependency with the `all` feature, and use `App::from_args` or `App::from_yaml` in a host-side binary or test:

```rust,ignore
// tests/run_on_host.rs — compiles with std
fn main() -> Result<(), PondError> {
    App::from_args(std::env::args_os())?
        .dispatch(my_no_std_pipeline)
}
```

This gives you visualization, validation, logging hooks, and parallel execution for free during development, without changing the pipeline itself.

## App::new

In `no_std` environments, `App::new` is the only available constructor:

```rust,ignore
let app = App::new(catalog, params);
app.execute(pipeline)?;
```

There is no YAML loading, no CLI parsing, and no `dispatch` — you construct the catalog and params directly and call `execute`.

## Default runner

The default runner tuple in `no_std` is `(SequentialRunner,)`. The `ParallelRunner` requires `std` (threads).

## Hooks in no_std

The `Hook` and `Hooks` traits work in `no_std`, but there are no built-in hook implementations (`LoggingHook` requires `std`). You can implement your own:

```rust,ignore
struct UartLogger;

impl Hook for UartLogger {
    fn before_node_run(&self, n: &dyn PipelineInfo) {
        // write to UART, toggle debug pin, etc.
        uart_print(n.name());
    }

    fn on_node_error(&self, n: &dyn PipelineInfo, _error: &str) {
        uart_print("ERROR: ");
        uart_print(n.name());
    }
}

App::new(catalog, params)
    .with_hooks((UartLogger,))
    .execute(pipeline)?;
```

Note that in `no_std`, the `error` string in `on_node_error` is always `"node error"` (not the full error message), since `Display` formatting requires allocation.

## Dataset names

In `no_std`, the catalog indexer is not available, so `DatasetRef::name` in hook callbacks is always `None`. Hooks can still use `ds.id` (pointer-based) or `ds.meta.type_string()` for identification.

## PondError in no_std

Only three `PondError` variants are available without `std`:

- `DatasetNotLoaded` — a dataset was read before being written
- `RunnerNotFound` — the specified runner name doesn't match any runner
- `CheckFailed` — pipeline validation failed (used by `dispatch` with `Command::Check`)

All other variants (`Io`, `Polars`, `SerdeYaml`, etc.) are feature-gated.

## Validation

`check()` works fully in `no_std`. It uses fixed-size stack arrays instead of `HashMap`:

```rust,ignore
let steps = pipeline(&catalog, &params);
steps.check()?;

// For pipelines with more than 20 datasets:
steps.check_with_capacity::<64>()?;
```
