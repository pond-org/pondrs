# Run

The `run` subcommand executes the pipeline.

## CLI usage

```sh
$ my_app run                              # default runner (sequential)
$ my_app run --runner parallel            # use parallel runner
$ my_app run --params threshold=0.8       # override parameters
$ my_app run --catalog output.path=/tmp/out.json  # override catalog
$ my_app run --nodes clean,summarize      # run only named nodes
$ my_app run --from-nodes clean           # run from a node onwards
$ my_app run --to-nodes report            # run up to a node
```

## Node filtering

You can run a subset of the pipeline by selecting specific nodes:

```sh
$ my_app run --nodes summarize,report       # run only these nodes
$ my_app run --from-nodes summarize         # run from this node onwards
$ my_app run --to-nodes report              # run up to and including this node
$ my_app run --from-nodes clean --to-nodes report  # run the subgraph between these nodes
```

`--nodes` and `--from-nodes`/`--to-nodes` are mutually exclusive. All flags accept comma-separated node names.

**`--nodes`** runs exactly the listed nodes, skipping everything else.

**`--from-nodes`** / **`--to-nodes`** computes the subgraph between the specified start and end nodes by following data dependencies. If only `--from-nodes` is given, all downstream nodes are included. If only `--to-nodes` is given, all upstream nodes are included.

Pipeline structure is preserved during filtering: if a sub-pipeline contains some matching nodes, it appears in the filtered run with only those nodes. Sub-pipelines with no matching nodes are dropped entirely.

If a specified node name doesn't exist in the pipeline, `PondError::NodeNotFound` is returned.

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
