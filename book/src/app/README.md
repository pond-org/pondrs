# App

The `App` struct bundles catalog, params, hooks, and runners together and provides methods for pipeline execution and CLI dispatch.

```rust,ignore
pub struct App<C, P, H = (), R = DefaultRunners> {
    catalog: C,
    params: P,
    hooks: H,
    runners: R,
    command: Command,
    // ...
}
```

`App` is generic over catalog, params, hooks, and runners. Builder methods return a new `App` with different type parameters, so the type system tracks what has been configured.

## Subcommands

`App::dispatch()` selects behavior based on the stored `Command`:

| Command | CLI | Description |
|---------|-----|-------------|
| `Command::Run` | `my_app run` | Execute the pipeline |
| `Command::Check` | `my_app check` | Validate pipeline structure |
| `Command::Viz` | `my_app viz` | Build graph and serve visualization |

## Chapter overview

- **[Initialization](./initialization.md)** — constructing an App (`from_yaml`, `from_args`, `new`)
- **[YAML Configuration](./yaml.md)** — catalog and params files, dot-notation overrides
- **[Run](./run.md)** — executing the pipeline
- **[Check](./check.md)** — validating pipeline structure via CLI
- **[Viz](./viz.md)** — interactive pipeline visualization
