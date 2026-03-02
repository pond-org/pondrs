# CLI Interface Design: `PondApp` Trait & Executable Builder

## Overview

This document describes the design for a trait-based interface that lets users build a full CLI
executable from their pipeline components with minimal boilerplate. The user specifies their
pipeline, catalog, params, and hooks by implementing a trait. The framework provides a `main()`
method that handles CLI parsing, YAML config loading, param overrides, and subcommand dispatch
(`run`, `check`, `viz`).

### Goals

- **Minimal boilerplate**: the user implements one trait, calls one function.
- **Minimal overhead**: no unnecessary allocations, fully monomorphized pipeline and hook dispatch,
  zero dynamic dispatch.
- **No macros**: the interface is a plain trait. The only nightly feature needed beyond what already
  exists is `impl_trait_in_assoc_type` for the pipeline GAT.
- **std-only**: the app module is gated on `#[cfg(feature = "std")]`. Existing no\_std code is
  unaffected.

### Non-Goals (for now)

- Multi-pipeline binaries (select pipeline by name). Single pipeline per binary.
- Full `viz` web server implementation. The infrastructure to build the graph and serve it is
  plumbed through; the actual web serving is a future step.

---

## The `PondApp` Trait

```rust
#[cfg(feature = "std")]
pub trait PondApp {
    /// The catalog struct containing all datasets. Must be deserializable from YAML
    /// (for path/config loading) and serializable (for the catalog indexer).
    type Catalog: Serialize + DeserializeOwned;

    /// The params struct containing all `Param<T>` fields. Must be deserializable
    /// from YAML and serializable. CLI overrides are applied via serde patching.
    type Params: Serialize + DeserializeOwned;

    /// The pipeline error type.
    type Error: From<PondError> + Send + Sync + Display + Debug + 'static;

    /// The pipeline type, parameterized by the borrow lifetime into catalog/params.
    /// Users write: `type Pipeline<'a> = impl Steps<Self::Error> + StepInfo;`
    /// The compiler infers the concrete type from `fn pipeline()`.
    type Pipeline<'a>: Steps<Self::Error> + StepInfo
        where Self::Catalog: 'a, Self::Params: 'a;

    /// Build the pipeline from catalog and params references.
    /// Required.
    fn pipeline<'a>(
        catalog: &'a Self::Catalog,
        params: &'a Self::Params,
    ) -> Self::Pipeline<'a>;

    /// Provide hooks for pipeline execution.
    /// Required. Return `()` for no hooks.
    fn hooks() -> impl Hooks;

    /// Optionally force a specific runner. If `None`, the CLI `--runner` flag
    /// is used (defaulting to sequential).
    fn runner() -> Option<RunnerChoice> { None }

    /// Path to the catalog YAML config file.
    fn catalog_path() -> &'static str { "conf/catalog.yml" }

    /// Path to the parameters YAML config file.
    fn params_path() -> &'static str { "conf/parameters.yml" }

    /// Provided method: full CLI entrypoint.
    /// Parses args, loads config, dispatches subcommand.
    fn main() { /* see Subcommand Dispatch section */ }
}
```

### Nightly Feature: `impl_trait_in_assoc_type`

This feature allows writing:

```rust
type Pipeline<'a> = impl Steps<Self::Error> + StepInfo;
```

inside a trait impl, letting the compiler infer the concrete pipeline type from `fn pipeline()`.
This avoids the user having to name the deeply nested generic type. The project already uses
nightly (`unboxed_closures`, `fn_traits`, `tuple_trait`), so one more feature gate is acceptable.

Add to `lib.rs`:
```rust
#![feature(impl_trait_in_assoc_type)]
```

---

## Runner Refactor (Path A: Hooks at Call Time)

### Motivation

Currently, runners carry hooks as a generic parameter (`SequentialRunner<H: Hooks>`). This couples
runner construction to hook types and prevents the app framework from independently selecting a
runner via CLI while injecting user-provided hooks.

### Change: `Hooks` Gets a `Sync` Supertrait

```rust
pub trait Hooks: Sync {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook));
}
```

Adding `Sync` as a supertrait means any type implementing `Hooks` is safe to share across threads.
This is necessary for `ParallelRunner` (which shares hooks across `thread::scope` threads) and
harmless for `SequentialRunner`. In practice, hook types are almost always `Sync` — they're
stateless or use `Mutex` for interior mutability. The existing tuple impls (`()`, `(H1,)`,
`(H1, H2)`, ...) automatically satisfy `Sync` when all elements are `Sync`.

`Sync` is in `core`, not `std`, so this does not affect no\_std compatibility.

### Change: `Runner::run` Accepts `&impl Hooks`

```rust
pub trait Runner {
    fn run<E>(
        &self,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + Display + Debug + 'static;
}
```

This is consistent with how `pipe`, `catalog`, and `params` are already passed (all `&impl Trait`).
Hooks are fully monomorphized — no vtable dispatch at the `Hooks` level. The `Sync` bound comes
from the `Hooks` supertrait, so it doesn't need to appear separately here.

### `SequentialRunner` Becomes a Unit Struct

```rust
pub struct SequentialRunner;

impl Runner for SequentialRunner {
    fn run<E>(
        &self,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Result<(), E> { /* ... */ }
}
```

Internally, everywhere that currently calls `self.hooks.for_each_hook(...)` changes to
`hooks.for_each_hook(...)`. The hook iteration logic is unchanged; only the source of the hooks
reference moves from `self` to the parameter.

### `ParallelRunner` Becomes a Unit Struct

Same transformation. The `Sync` supertrait on `Hooks` ensures hooks can be shared across threads
in `thread::scope`. `ParallelRunner` no longer needs a generic parameter.

```rust
pub struct ParallelRunner;
```

### no\_std Compatibility

- `&impl Hooks` is monomorphized — no fat pointer, no allocation, works in no\_std.
- `Sync` is in `core`, not `std`.
- The no\_std `SequentialRunner::run` variant (the `#[cfg(not(feature = "std"))]` branch) gets the
  same signature change.

### `RunnerChoice` Enum

```rust
pub enum RunnerChoice {
    Sequential,
    Parallel,
}
```

Used by `PondApp::runner()` and the CLI `--runner` flag. The framework matches on this enum and
constructs the appropriate concrete runner. This is a single branch — no dynamic dispatch on the
runner itself.

---

## Hooks: Fully Monomorphized

### No Type Erasure Needed

Since the app framework uses `RunnerChoice` enum dispatch (not `dyn Runner`), the hooks type is
always known at the call site:

```rust
let hooks = Self::hooks();  // concrete type, known at compile time

match runner_choice {
    RunnerChoice::Sequential => SequentialRunner.run(&pipeline, &catalog, &params, &hooks),
    RunnerChoice::Parallel => ParallelRunner.run(&pipeline, &catalog, &params, &hooks),
}
```

Both arms monomorphize `run()` with the concrete hooks type. No boxing, no vtable dispatch at the
`Hooks` level.

### How It Flows

1. User implements `fn hooks() -> impl Hooks { (LoggingHook::new(),) }`
2. The provided `main()` calls `let hooks = Self::hooks();`
3. Hooks live on the stack as their concrete tuple type.
4. Passed to the runner as `&hooks` — fully monomorphized, the compiler inlines through
   `for_each_hook` into direct calls to each hook method.

### Cost

Zero overhead beyond what the hook methods themselves do. Each individual hook call inside
`for_each_hook` dispatches through `&dyn Hook` (this is the existing behavior from the `Hooks`
tuple impls and is unchanged). Hook events fire at I/O boundaries (before/after node runs,
dataset loads), so even this existing vtable cost is invisible.

The only compile-time cost is one extra monomorphization axis per distinct hooks tuple type. In
practice there's one hooks combination per binary, so this is a single copy.

---

## CLI Structure

Using `clap` (derive mode), gated on the `std` feature.

```
myapp <SUBCOMMAND> [OPTIONS]

Subcommands:
    run     Execute the pipeline
    check   Validate pipeline structure (dependency ordering, output uniqueness)
    viz     Build pipeline graph and serve visualization

Global options:
    --catalog-path <PATH>    Path to catalog YAML [default: conf/catalog.yml]
    --params-path <PATH>     Path to parameters YAML [default: conf/parameters.yml]

Run options:
    --runner <sequential|parallel>   Runner to use [default: sequential]
    --params <KEY=VALUE>...          Override parameter values (dot notation for nesting)

Viz options:
    --port <PORT>                    Port for the visualization server [default: 8080]
    --output <PATH>                  Write pipeline graph JSON to file instead of serving
```

### Param Override Syntax

Dot notation for nested fields:
```
myapp run --params model.learning_rate=0.01 --params training.batch_size=64
```

Values are parsed as YAML scalars (auto-detecting numbers, bools, strings, null).

### Dependency

```toml
clap = { version = "4", features = ["derive"], optional = true }
```

Gated on `std` feature. The `clap` types do not leak into the `PondApp` trait — the trait uses
`RunnerChoice` and `&'static str` for paths, keeping the public API decoupled from the arg parser.

---

## YAML Configuration & Param Overrides

### Loading Strategy

1. Read YAML file into `serde_yaml::Value` (a generic tree).
2. For params: apply CLI `--params` overrides by walking the `Value` tree with dot-split keys.
3. Deserialize the (possibly patched) `Value` into the concrete `Self::Catalog` / `Self::Params`.

### Param Patching

```rust
fn apply_overrides(value: &mut serde_yaml::Value, overrides: &[(String, String)]) {
    for (dotted_key, raw_val) in overrides {
        let parts: Vec<&str> = dotted_key.split('.').collect();
        let mut current = value;
        for part in &parts[..parts.len() - 1] {
            current = current.get_mut(part).expect("...");
        }
        let last = parts.last().unwrap();
        // Parse as YAML scalar to auto-detect type (number, bool, string, null)
        let parsed = serde_yaml::from_str(raw_val)
            .unwrap_or(serde_yaml::Value::String(raw_val.clone()));
        current[last] = parsed;
    }
}
```

This works with `Param<T>` transparently because `Param<T>` serializes/deserializes as just `T`
(transparent serde wrapper). A YAML file like:

```yaml
model:
  learning_rate: 0.001
```

with override `--params model.learning_rate=0.01` patches the tree, then deserializes into
`MyParams` where `learning_rate: Param<f64>` picks up `0.01`.

### File Absence

If a YAML file does not exist:
- **Params**: error with a clear message pointing to the expected path.
- **Catalog**: error with a clear message.

The user can override paths via `--catalog-path` / `--params-path` or by overriding the trait's
`catalog_path()` / `params_path()` methods.

Optionally, we can also look into supporting `Default` as a fallback when files are missing, but
this is not required for the initial implementation.

---

## Subcommand Behavior

### `run`

1. Parse CLI args.
2. Load catalog YAML → deserialize into `Self::Catalog`.
3. Load params YAML → apply `--params` overrides → deserialize into `Self::Params`.
4. Build pipeline: `Self::pipeline(&catalog, &params)`.
5. Build hooks: `Self::hooks()`.
6. Determine runner: `Self::runner().unwrap_or(cli_runner_arg)`, default `Sequential`.
7. Execute:
   ```rust
   match runner_choice {
       RunnerChoice::Sequential => SequentialRunner.run(&pipeline, &catalog, &params, &hooks)?,
       RunnerChoice::Parallel => ParallelRunner.run(&pipeline, &catalog, &params, &hooks)?,
   }
   ```
8. Exit 0 on success, print error and exit 1 on failure.

### `check`

1. Load catalog and params (same as `run`, including any `--params` overrides).
2. Build pipeline.
3. Call `pipeline.check()`.
4. On success: print validation summary (number of nodes, datasets, etc.), exit 0.
5. On failure: print `CheckError` details (which node, which dataset), exit 1.

Note: `check` does not execute the pipeline. Dataset files don't need to contain valid data, but
the catalog YAML must exist so the struct can be deserialized (dataset path fields need values, even
if the files at those paths don't exist).

### `viz`

1. Load catalog and params.
2. Build pipeline.
3. Build graph: `build_pipeline_graph(&pipeline, &catalog, &params)`.
4. The `PipelineGraph` contains:
   - `nodes: Vec<GraphNode>` — all nodes with name, leaf flag, inputs, outputs, parent info
   - `edges: Vec<Edge>` — data flow edges between leaf nodes
   - `node_indices: Vec<usize>` — indices of leaf (executable) nodes
   - `source_datasets: HashSet<usize>` — external inputs (params, pre-loaded data)
   - `dataset_names: HashMap<usize, String>` — human-readable names from catalog indexer
5. If `--output <path>` is given: serialize graph to JSON, write to file, exit 0.
6. Otherwise: start a web server on `--port` serving the graph data. (Implementation of the actual
   web UI is a future step; the infrastructure to produce and serve the graph is what this plan
   covers.)

---

## Dependencies & Feature Gates

### New Dependencies

```toml
clap = { version = "4", features = ["derive"], optional = true }
serde_json = { version = "1", optional = true }  # for viz JSON output
```

Both gated on `std`:
```toml
std = ["serde/std", "dep:serde_yaml", "dep:log", "dep:clap", "dep:serde_json"]
```

### New Nightly Features

```rust
#![feature(impl_trait_in_assoc_type)]
```

Added to `lib.rs` alongside the existing `unboxed_closures`, `fn_traits`, `tuple_trait`.

### Module Structure

```
src/
  app/
    mod.rs          — PondApp trait, RunnerChoice, re-exports
    cli.rs          — clap arg definitions (CliArgs, Subcommand enums)
    config.rs       — YAML loading, param patching (apply_overrides)
  lib.rs            — add `pub mod app;` gated on std
```

The `app` module is `#[cfg(feature = "std")]` and does not affect no\_std compilation.

---

## Implementation Phases

### Phase 1: Runner Refactor

- Add `Sync` supertrait to `Hooks`.
- Change `Runner::run` signature to accept `hooks: &impl Hooks`.
- Convert `SequentialRunner` to unit struct, move hook calls to use the parameter.
- Convert `ParallelRunner` to unit struct, same transformation.
- Update `main.rs` example and all tests to pass hooks to `run()`.
- Verify no\_std example still compiles.

### Phase 2: App Module — Trait & Config Loading

- Add `clap` and `serde_json` dependencies.
- Add `impl_trait_in_assoc_type` feature gate.
- Create `src/app/` module with `PondApp` trait definition.
- Implement YAML loading for catalog and params.
- Implement `apply_overrides` for param patching.
- Implement CLI arg parsing with clap.

### Phase 3: Subcommand Dispatch

- Implement the provided `PondApp::main()` method.
- Wire up `run` subcommand (runner selection, pipeline execution).
- Wire up `check` subcommand (validation, formatted output).
- Wire up `viz` subcommand (graph building, JSON output).
- Error handling: print to stderr, exit codes.

### Phase 4: Integration Testing

- Create a test binary that implements `PondApp`.
- Test `run` with sequential and parallel runners.
- Test `check` with valid and invalid pipelines.
- Test `--params` overrides with nested keys.
- Test `viz --output` JSON output.

---

## Example: Full User Code

```rust
use pondrs::prelude::*;  // or individual imports
use pondrs::app::{PondApp, RunnerChoice};

// --- Catalog: dataset definitions ---
#[derive(Serialize, Deserialize)]
struct MyCatalog {
    raw_data: PolarsCsvDataset,
    processed: MemoryDataset<DataFrame>,
    output: PolarsCsvDataset,
}

// --- Params: configuration values ---
#[derive(Serialize, Deserialize)]
struct MyParams {
    threshold: Param<f64>,
    model: ModelParams,
}

#[derive(Serialize, Deserialize)]
struct ModelParams {
    learning_rate: Param<f64>,
    epochs: Param<usize>,
}

// --- Pipeline construction ---
fn build_pipeline<'a>(
    cat: &'a MyCatalog,
    params: &'a MyParams,
) -> impl Steps<PondError> + StepInfo + 'a {
    (
        Node {
            name: "load_and_filter",
            func: |df: DataFrame, threshold: f64| -> Result<(DataFrame,), PondError> {
                // ... processing ...
                Ok((df,))
            },
            input: (&cat.raw_data, &params.threshold),
            output: (&cat.processed,),
        },
        Node {
            name: "train_and_save",
            func: |df: DataFrame, lr: f64, epochs: usize| -> Result<(DataFrame,), PondError> {
                // ... training ...
                Ok((df,))
            },
            input: (&cat.processed, &params.model.learning_rate, &params.model.epochs),
            output: (&cat.output,),
        },
    )
}

// --- App definition ---
struct MyApp;

impl PondApp for MyApp {
    type Catalog = MyCatalog;
    type Params = MyParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error> + StepInfo;

    fn pipeline<'a>(cat: &'a MyCatalog, params: &'a MyParams) -> Self::Pipeline<'a> {
        build_pipeline(cat, params)
    }

    fn hooks() -> impl Hooks {
        (LoggingHook::new(),)
    }

    // Optional: force parallel runner (otherwise CLI --runner flag is used)
    // fn runner() -> Option<RunnerChoice> { Some(RunnerChoice::Parallel) }
}

fn main() {
    MyApp::main();
}
```

### CLI Usage

```bash
# Run with default config
myapp run

# Run with param overrides
myapp run --params model.learning_rate=0.01 --params threshold=0.5

# Run with parallel runner
myapp run --runner parallel

# Validate pipeline structure
myapp check

# Export pipeline graph
myapp viz --output pipeline.json

# Custom config paths
myapp run --catalog-path conf/production/catalog.yml --params-path conf/production/parameters.yml
```

---

## Open Questions

1. **`check` without YAML files**: Should we support `Default` as a fallback for catalog/params
   when YAML files are missing? This would let `check` work before config files exist. Adds a
   `Default` bound to the trait.

2. **Catalog overrides**: Should `--catalog <key=value>` be supported for overriding dataset paths
   from the CLI, using the same serde patching mechanism as params? This is useful but adds surface
   area.

3. **Environment-based config**: Kedro-style `conf/base/` + `conf/local/` layering. Worth
   designing for but probably not implementing in the first pass.

4. **`viz` web framework**: When we implement the actual web server for `viz`, should it be a
   separate optional dependency (e.g. `axum`, `warp`) gated on a `viz` feature, or a minimal
   hand-rolled HTTP server?
