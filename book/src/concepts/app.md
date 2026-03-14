# App

The `App` struct ties everything together — catalog, params, hooks, and runners — and provides CLI dispatch for running, validating, and visualizing your pipeline.

## In the minimal example

```rust,ignore
{{#include ../../../examples/minimal.rs:app}}
```

`App::from_yaml` loads the catalog and params from YAML files, then `with_args` parses CLI arguments for subcommand selection and parameter overrides. Finally, `dispatch` runs the appropriate subcommand (`run`, `check`, or `viz`).

## Subcommands

```sh
$ my_app run                              # execute the pipeline
$ my_app run --params threshold=0.8       # override params from CLI
$ my_app check                            # validate pipeline DAG
$ my_app viz                              # interactive pipeline visualization
```

For the full details on the `App` struct, initialization options, YAML configuration, and subcommands, see the [App](../app/README.md) chapter.
