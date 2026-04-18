# Check

The `check` subcommand validates the pipeline structure without executing it.

## CLI usage

```sh
$ my_app check
Pipeline is valid.
```

If validation fails:

```sh
$ my_app check
Error: Pipeline validation failed:
  - Node 'process' requires dataset 0x7f8a..., which is produced by a later node
```

## What it validates

`check` calls `PipelineInfo::check()` on the pipeline, which verifies:

- **Sequential ordering** — no node reads a dataset produced by a later node
- **No duplicate outputs** — each dataset is produced by at most one node
- **Params are read-only** — no node writes to a `Param`
- **Pipeline contracts** — `Pipeline` declared inputs/outputs match children

See [Check](../pipelines/check.md) for the full list of `CheckError` variants.

## Programmatic use

You can call `check()` directly on any `PipelineInfo`:

```rust,ignore
let steps = pipeline(&catalog, &params);
match steps.check() {
    Ok(()) => println!("Valid!"),
    Err(e) => eprintln!("Error: {e}"),
}
```

Or via `App::dispatch`, which calls `check()` when the command is `Command::Check`:

```rust,ignore
App::new(catalog, params)
    .with_command(Command::Check)
    .dispatch(pipeline)?;
```
