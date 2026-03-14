# Run

The `run` subcommand executes the pipeline.

## CLI usage

```sh
$ my_app run                              # default runner (sequential)
$ my_app run --runner parallel            # use parallel runner
$ my_app run --params threshold=0.8       # override parameters
$ my_app run --catalog output.path=/tmp/out.json  # override catalog
```

## How it works

1. The pipeline function is called with `(&catalog, &params)` to construct the steps
2. The selected runner (by `--runner` flag or default) is looked up by name
3. The runner executes the steps, firing hooks at each lifecycle point
4. If any node fails, execution stops and the error is returned

## `execute` vs `dispatch`

- `app.execute(pipeline_fn)` — always runs the pipeline, regardless of the stored command
- `app.dispatch(pipeline_fn)` — checks the stored command and dispatches to run, check, or viz

Use `execute` in tests or when you want to run the pipeline directly. Use `dispatch` in CLI apps to support all subcommands.

## Runner selection

The `--runner` flag selects a runner by name. Available runners depend on what's configured:

```rust,ignore
// Default: sequential + parallel (std), sequential only (no_std)
App::from_yaml(..)?
    .dispatch(pipeline)?;

// Custom runners
App::from_yaml(..)?
    .with_runners((SequentialRunner, ParallelRunner, MyRunner))
    .dispatch(pipeline)?;
```

If the named runner isn't found, `PondError::RunnerNotFound` is returned.

## Post-run inspection

When using `App::new` + `execute`, you can inspect datasets after the pipeline runs:

```rust,ignore
let app = App::new(catalog, params);
app.execute(pipeline)?;

// Access results
let summary: f64 = app.catalog().summary.load()?;
assert!(summary > 0.0);
```

This pattern is useful for integration tests.
