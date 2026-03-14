# Initialization

There are several ways to construct an `App`, depending on whether your catalog comes from YAML files or is built programmatically.

## `App::from_yaml` + `with_args`

The most common pattern for file-based pipelines. Loads catalog and params from YAML, then applies CLI arguments:

```rust,ignore
App::from_yaml("conf/catalog.yml", "conf/params.yml")?
    .with_args(std::env::args_os())?
    .dispatch(pipeline)
```

`from_yaml` loads and deserializes both files. `with_args` parses CLI arguments for subcommand selection and parameter overrides.

*Requires `std`. Catalog and params types must implement `DeserializeOwned`.*

## `App::from_args`

Parses CLI arguments including `--catalog-path` and `--params-path` flags:

```rust,ignore
App::from_args(std::env::args_os())?
    .dispatch(pipeline)
```

Default paths if not specified: `conf/base/catalog.yml` and `conf/base/parameters.yml`.

*Requires `std`.*

## `App::from_cli`

Same as `from_args` but takes a pre-parsed `CliArgs` struct:

```rust,ignore
use pondrs::app::cli::CliArgs;
use clap::Parser;

let cli = CliArgs::parse();
App::from_cli(cli)?
    .dispatch(pipeline)
```

## `App::new`

Construct with catalog and params directly — no YAML, no CLI:

```rust,ignore
let catalog = Catalog { /* ... */ };
let params = Params { /* ... */ };

App::new(catalog, params)
    .execute(pipeline)?;
```

This is the only constructor available in `no_std`. It defaults to `Command::Run` with no hooks and the default runners.

Use `App::new` when:
- The catalog is built programmatically (e.g. `RegisterDataset`, `GpioDataset`)
- You're writing tests and want to inspect datasets after execution
- You're in a `no_std` environment

You can still add CLI support to a programmatic catalog via `with_args`:

```rust,ignore
App::new(catalog, params)
    .with_args(std::env::args_os())?  // adds subcommand + param overrides
    .dispatch(pipeline)?;
```

## Builder methods

After construction, configure the app with builder methods:

```rust,ignore
App::from_yaml("catalog.yml", "params.yml")?
    .with_args(std::env::args_os())?
    .with_hooks((LoggingHook::new(),))
    .with_runners((SequentialRunner, ParallelRunner))
    .dispatch(pipeline)?;
```

| Method | Description |
|--------|-------------|
| `with_hooks(h)` | Set the hooks tuple |
| `with_runners(r)` | Set the runners tuple |
| `with_command(cmd)` | Set the command directly |
| `with_args(iter)` | Parse CLI args, apply command + param overrides |
| `with_cli(cli)` | Apply pre-parsed `CliArgs` |

## Execution methods

| Method | Description |
|--------|-------------|
| `execute(f)` | Run the pipeline directly (always `Command::Run` behavior) |
| `dispatch(f)` | Choose behavior based on stored `Command` (run/check/viz) |
| `catalog()` | Borrow the catalog (useful in tests) |
| `params()` | Borrow the params |
