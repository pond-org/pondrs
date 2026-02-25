# CLI + Web Visualization for pondrs

## Context

The pondrs library has a complete pipeline framework (nodes, pipelines, runners, hooks, datasets, catalog indexing) but no user-facing interface. Users currently wire everything together manually in `main.rs`. The goal is to add:

1. **`pondrs check`** — CLI command to validate that pipelines are well-formed without executing them
2. **`pondrs run`** — CLI command to execute a pipeline with optional parameter overrides
3. **`pondrs viz`** — CLI command that launches a web server for pipeline visualization (React frontend, axum backend, WebSocket for live updates)

This follows the two-process architecture from `docs/pipeline-viz-design.md`.

---

## User-Facing API

The library user's `main.rs` becomes:

```rust
fn main() {
    PondrsApp::<Catalog, Parameters>::builder()
        .params_path("conf/parameters.yml")
        .catalog_path("conf/catalog.yml")
        .pipeline("pipe1", construct_pipe1)
        .pipeline("pipe2", construct_pipe2)
        .hook(LoggingHook)
        .build()
        .run();  // parses CLI args and dispatches to check/run/viz
}
```

The `.hook()` method adds user-provided hooks that run alongside the internal `VizHook`. Hooks are collected into a `Vec<Box<dyn Hook + Send + Sync>>` to support an arbitrary number (replacing the current tuple-of-hooks approach for the app builder, while the tuple approach remains available for direct runner usage).

**CLI usage:**
```
pondrs check                                                   # validate all pipelines
pondrs check --pipeline pipe1                                  # validate specific pipeline
pondrs run --pipeline pipe1 --parallel --params initial_value=10
pondrs viz --port 3000 --pipeline pipe1
```

Requirements on user types: `Catalog` and `Parameters` must derive `Serialize + Deserialize + Clone`. The YAML files at the configured paths must exist and contain valid serializations of these types.

---

## Key Design Decisions

### Type Erasure for Pipeline Registry

Each pipeline function returns a different `impl Steps` type. The `Steps` trait has a `Tuple` bound, making it non-object-safe (`dyn Steps` is impossible). Solution: **closure-based erasure**.

At registration time, `.pipeline("name", f)` creates closures that capture the concrete `S` type internally but are stored as trait objects:

```rust
struct PipelineRegistration {
    name: String,
    // Constructs pipeline, extracts graph, drops pipeline
    extract_graph_fn: Arc<dyn Fn(&P, &C, &CatalogIndex) -> PipelineGraph + Send + Sync>,
    // Constructs pipeline, runs it with hook, drops pipeline
    run_fn: Arc<dyn Fn(&P, &C, &dyn Hooks) -> Result<(), String> + Send + Sync>,
}
```

The pipeline is constructed and consumed within the closure body, never stored. `Arc` so the closure can be shared with spawned threads (e.g., the viz WebSocket handler).

The `register_pipeline` helper monomorphizes at call time:

```rust
fn register_pipeline<C, P, S, F>(name: &str, f: F) -> PipelineRegistration<C, P>
where
    S: Steps,
    F: Fn(&P, &C) -> S + Send + Sync + Clone + 'static,
{
    let f_graph = f.clone();
    let f_run = f;
    PipelineRegistration {
        name: name.to_string(),
        extract_graph_fn: Arc::new(move |params, catalog, cat_idx| {
            let steps = f_graph(params, catalog);
            extract_graph(&steps, cat_idx)
        }),
        run_fn: Arc::new(move |params, catalog, hooks| {
            let steps = f_run(params, catalog);
            // run with runner + hooks
        }),
    }
}
```

### YAML Loading

- Load params from `conf/parameters.yml` via `serde_yaml::from_str` — file must exist
- Load catalog from `conf/catalog.yml` via `serde_yaml::from_str` — file must exist
- `MemoryDataset` fields have `#[serde(skip)]` on their value, so they deserialize to `Default` (empty) — correct since they're intermediate storage
- `Param<T>` serializes as the inner value (newtype), so YAML files just contain raw values
- No default params/catalog — the YAML files are the source of truth

### Parameter Overrides (CLI)

`--params key=value,key2=value2` on the CLI. Implementation: serialize current params to `serde_yaml::Value`, navigate by dot-separated key path, replace the value, deserialize back to `P`. Works with any structure depth because it operates on the dynamic YAML tree.

### Pipeline Validation (`check` command)

The `check` command constructs each pipeline (without executing it), extracts the graph, and validates:

1. **Connectivity** — every node's inputs are produced by some other node or are source datasets (params). Detects dangling inputs.
2. **No cycles** — the DAG has a valid topological ordering.
3. **No orphan nodes** — every node is reachable from sources and leads to a sink.
4. **Dataset naming** — all dataset pointer IDs resolve to names via `CatalogIndex`. Detects references to datasets not in the catalog.
5. **Duplicate names** — no two nodes share the same name within a pipeline.

The check constructs the pipeline by calling the user's pipeline function with the loaded params + catalog (same as `run`/`viz`), then uses `graph::extract` to build the DAG and runs the validation. No node `call()` methods are invoked — only the structural introspection methods (`for_each_child`, `input_dataset_ids`, `output_dataset_ids`, `get_name`).

Output: prints a summary per pipeline (node count, edge count, any warnings/errors) and exits with code 0 on success, 1 on failure.

### VizHook for Live Updates

A `VizHook` sends execution events to the viz web server over WebSocket. If the viz server isn't running, the hook logs a warning and no-ops. Uses `Mutex<Sender>` or `tungstenite` WebSocket client.

---

## Implementation Phases

### Phase 1: Fix Node/Pipeline naming (prerequisite)

Currently `Node::get_name()` at `src/core/node.rs:28` returns `std::any::type_name::<F>()` (compiler closure type name), not the user-supplied `name` field. `Pipeline` at `src/core/pipeline.rs` has no `name` field at all.

| File | Change |
|------|--------|
| `src/core/node.rs:28` | Change `get_name()` to return `self.name` instead of `type_name::<F>()` |
| `src/core/pipeline.rs` | Add `pub name: &'static str` field to `Pipeline` struct; return `self.name` from `get_name()`; add `input_dataset_ids()` and `output_dataset_ids()` methods |
| `src/main.rs` | Add `name:` field to all `Pipeline` literals |

### Phase 2: Graph extraction (`src/graph/`)

New module that walks the `PipelineItem` tree and produces a serializable DAG. This is used by both the viz web API (serves the graph as JSON) and potentially CLI debug output.

| File | Contents |
|------|----------|
| `src/graph/mod.rs` | Module declarations, re-exports |
| `src/graph/types.rs` | `GraphNode { id, name, node_type, parent, inputs, outputs }`, `GraphEdge { from_node, to_node, dataset_name }`, `PipelineGraph { nodes, edges }` — all derive `Serialize` for JSON API |
| `src/graph/extract.rs` | `extract_graph(pipe: &impl Steps, catalog_index: &CatalogIndex) -> PipelineGraph` — walks tree via `for_each_child`, builds edges by matching output→input dataset IDs |
| `src/graph/validate.rs` | `validate_graph(graph: &PipelineGraph) -> Vec<ValidationError>` — checks connectivity, cycles, orphans, duplicate names |

Reuses: `PipelineItem::for_each_child`, `input_dataset_ids()`, `output_dataset_ids()`, `CatalogIndex`.

### Phase 3: VizHook (`src/hooks/viz.rs`)

A hook that pushes execution events to the viz web server over WebSocket:

```rust
pub struct VizHook {
    tx: Mutex<Option<WebSocketSender>>,  // None if viz server not reachable
}
```

Events sent as JSON:
```rust
struct VizEvent {
    event_type: String,  // "node_started", "node_completed", "node_failed", etc.
    name: String,
    timestamp_ms: u64,
    duration_ms: Option<u64>,
    error: Option<String>,
}
```

Implements all `Hook` trait methods. If the WebSocket connection fails at startup, the hook silently degrades (no updates, no crash).

### Phase 4: CLI + App builder (`src/app/`)

| File | Contents |
|------|----------|
| `src/app/mod.rs` | Module declarations, `pub use builder::PondrsApp` |
| `src/app/builder.rs` | `PondrsApp<C, P>` builder with `.params_path()`, `.catalog_path()`, `.pipeline()`, `.hook()`, `.build()`, `.run()`. Internal `PipelineRegistration` with erased closures |
| `src/app/cli.rs` | CLI argument parsing with `clap`: `check` subcommand (--pipeline), `run` subcommand (--pipeline, --parallel, --params, --viz-url), and `viz` subcommand (--port, --pipeline) |
| `src/app/params.rs` | YAML loading, parameter override logic (serde_yaml round-trip) |

The `.run()` method:
1. Parses CLI args via clap
2. Dispatches to `check_command()`, `run_command()`, or `viz_command()`
3. `check_command()`: loads params + catalog, constructs pipeline(s), extracts graph, validates structure, prints report
4. `run_command()`: loads params (YAML + overrides), loads catalog, constructs and runs the selected pipeline
5. `viz_command()`: loads params + catalog, extracts graph, starts the web server

### Phase 5: Viz web server (`src/viz/`)

| File | Contents |
|------|----------|
| `src/viz/mod.rs` | Module declarations |
| `src/viz/server.rs` | Axum server setup: routes, shared state, `run_server()` |
| `src/viz/api.rs` | REST endpoints: `GET /api/pipeline` (graph JSON), `GET /api/catalog` (catalog entries), `GET /api/params` (parameter values) |
| `src/viz/ws.rs` | WebSocket endpoint `/ws`: broadcasts execution events from `VizHook` to all connected browser clients |
| `src/viz/state.rs` | Shared server state: pipeline graph, catalog data, param data, broadcast channel for WS events |

**REST API:**
- `GET /api/pipeline?name=pipe1` → `PipelineGraph` as JSON (nodes + edges)
- `GET /api/pipelines` → list of registered pipeline names
- `GET /api/catalog` → list of catalog entries (name, type, path if file-based)
- `GET /api/params` → current parameter values as flat key-value pairs

**WebSocket `/ws`:**
- Server broadcasts `VizEvent` JSON messages as they arrive from `run` commands
- Client connects and receives live updates for the execution timeline

**Static files:**
- For now, serve from a `frontend/dist/` directory
- Later, embed with `rust-embed` for single-binary distribution

### Phase 6: Frontend (TypeScript/React)

| Path | Contents |
|------|----------|
| `frontend/` | Vite + React + TypeScript project |
| `frontend/src/App.tsx` | Main layout: sidebar, graph view, detail panel, timeline |
| `frontend/src/components/PipelineGraph.tsx` | React Flow DAG visualization with ELK layout |
| `frontend/src/components/CatalogBrowser.tsx` | Tree view of datasets |
| `frontend/src/components/NodeDetail.tsx` | Selected node info panel |
| `frontend/src/components/ExecutionTimeline.tsx` | Gantt chart of node execution |
| `frontend/src/hooks/useWebSocket.ts` | WebSocket connection + reconnection logic |
| `frontend/src/api.ts` | REST API client (fetch wrappers) |

Libraries: React 19, Vite, `@xyflow/react` (React Flow v12), `elkjs`, `shadcn/ui`, Tailwind CSS v4.

### Phase 7: Integration

| File | Change |
|------|--------|
| `src/lib.rs` | Add `pub mod app; pub mod graph; pub mod viz;` |
| `Cargo.toml` | Add `clap`, `axum`, `tokio`, `tower-http`, `serde_json`, `tokio-tungstenite` dependencies |
| `src/hooks/mod.rs` | Add `pub mod viz;` |
| `src/main.rs` | Rewrite to use `PondrsApp::builder()...build().run()` |

---

## New Dependencies

| Crate | Purpose |
|-------|---------|
| `clap = { version = "4", features = ["derive"] }` | CLI argument parsing |
| `axum = "0.8"` | Web framework (REST + WebSocket) |
| `tokio = { version = "1", features = ["full"] }` | Async runtime for axum |
| `tower-http = { version = "0.6", features = ["fs", "cors"] }` | Static file serving, CORS middleware |
| `serde_json = "1.0"` | JSON serialization for API responses |
| `tokio-tungstenite = "0.26"` | WebSocket client (VizHook connecting to viz server) |

---

## Files to Create/Modify

**New files (~15 Rust + frontend):**
- `src/graph/mod.rs`, `src/graph/types.rs`, `src/graph/extract.rs`, `src/graph/validate.rs`
- `src/hooks/viz.rs`
- `src/app/mod.rs`, `src/app/builder.rs`, `src/app/cli.rs`, `src/app/params.rs`
- `src/viz/mod.rs`, `src/viz/server.rs`, `src/viz/api.rs`, `src/viz/ws.rs`, `src/viz/state.rs`
- `frontend/` directory (React project)

**Modified files (5):**
- `Cargo.toml` — add dependencies
- `src/lib.rs` — add module declarations
- `src/core/node.rs` — fix `get_name()` to return `self.name`
- `src/core/pipeline.rs` — add `name` field, fix `get_name()`, add dataset ID methods
- `src/main.rs` — rewrite to use PondrsApp builder

---

## Verification

1. `cargo build` — compiles without errors
2. `pondrs check` — validates all pipelines, prints summary, exits 0
3. `pondrs check --pipeline pipe1` — validates only pipe1
4. `pondrs run --pipeline pipe1` — runs pipe1 with default params, prints output
5. `pondrs run --pipeline pipe1 --params initial_value=10` — runs with override, output reflects new value
6. `pondrs run --pipeline pipe2 --parallel` — runs pipe2 with parallel runner
7. `pondrs viz --port 3000` — starts web server, opens browser to `http://localhost:3000`
8. Web UI shows pipeline DAG for pipe1 with correct node names and edges
9. Web UI catalog browser shows all datasets with types
10. Web UI params panel shows `initial_value: 2`
11. While viz is running, `pondrs run --pipeline pipe1 --viz-url ws://localhost:3000/ws` — web UI shows live execution progress
12. Create `conf/parameters.yml` with `initial_value: 42` — `pondrs run` picks it up
