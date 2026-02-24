# Pipeline Visualization Interface — Design Notes

## Overview

A web interface that visualizes pipeline structure and execution status. Works with both the sequential and parallel runners. Active monitoring is implemented using the hook system. Backend in Rust, frontend in TypeScript.

Two separate commands:
- **`pondrs viz`** — launches the web UI. Shows pipeline structure, catalog browsing, dataset previews. Works standalone, without any pipeline running.
- **`pondrs run`** — executes a pipeline. A `VizHook` pushes real-time status updates to the viz server (if running) over WebSocket.

---

## Architecture

### Two-Process Model

```
┌─────────────────┐         ┌──────────────────────────┐
│   pondrs run    │         │     pondrs viz            │
│                 │  WS/HTTP│                           │
│  Pipeline exec  │────────►│  HTTP Server              │
│  + VizHook      │         │  ├─ REST API (structure)  │
│                 │◄────────│  ├─ WebSocket (live)      │
└─────────────────┘         │  └─ Static files (SPA)    │
                            └──────────────────────────┘
```

- **`viz` command**: Starts an HTTP server. Serves the frontend SPA. Exposes a REST endpoint for pipeline structure (`GET /api/pipeline`). Maintains a WebSocket endpoint for real-time updates (`/ws`). Can run standalone — pipeline graph is visible, catalog is browsable, all nodes show as "idle".
- **`run` command**: Executes the pipeline with runners. A `VizHook` connects to the viz server (if running) and pushes status events over WebSocket. If viz isn't running, execution proceeds normally (the hook no-ops or logs a warning).

### Pipeline Structure Extraction

Need a function that walks the `PipelineItem` tree and produces a serializable DAG representation:

```rust
struct PipelineGraph {
    nodes: Vec<GraphNode>,    // id, name, is_leaf, parent_pipeline
    edges: Vec<GraphEdge>,    // from_node, to_node, dataset_id
    groups: Vec<GraphGroup>,  // nested pipeline containers
}
```

The `PipelineItem` trait already provides `for_each_child`, `get_name`, `input_dataset_ids`, `output_dataset_ids` — enough to build this. The parallel runner already does similar work (collecting leaves, building dependency sets). This logic should be extracted into a shared utility.

### Pipeline Registry

The `viz` command needs access to pipeline structure without executing it. This requires a way to construct the pipeline graph (catalog + parameters + pipeline definition) and serialize it, independent of any runner. The same pipeline definition function is called by both `run` (to execute) and `viz` (to inspect).

### Frontend Bundling

Two modes:
- **Build-time embedding** using `rust-embed` — compiled frontend assets baked into the Rust binary. Single binary distribution.
- **Separate dev server** during development — Rust server proxies to Vite's dev server for hot reload.

---

## Dataset Naming via Serde Introspection

### The Problem

Dataset identity is pointer-based (`ptr_to_id` casts `&T` to `*const () as usize`). These IDs are ephemeral and meaningless to a user. The viz needs human-readable names like `"a"`, `"iris.input"`, etc.

### The Solution

The catalog structs already derive `Serialize`. When serde's generated code calls `serialize_field("a", &self.a)`, the `value` parameter **is** `&self.a` — the exact same pointer the pipeline nodes use. So `value as *const T as *const () as usize` produces the **same ID** as `ptr_to_id`.

Write a custom "extractor" `Serializer` that doesn't actually serialize — it just captures `(field_name, pointer_id)` pairs:

```rust
struct DatasetNameExtractor {
    names: HashMap<usize, String>,
    prefix: String,  // for nested structs: "iris.input"
}

impl SerializeStruct for &mut DatasetNameExtractor {
    type Ok = ();
    type Error = /* ... */;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let ptr_id = value as *const T as *const () as usize;
        let full_name = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", self.prefix, key)
        };
        self.names.insert(ptr_id, full_name);
        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}
```

Usage:

```rust
let catalog = Catalog { a: MemoryDataset::new(), /* ... */ };

// Extract name mapping
let mut extractor = DatasetNameExtractor::new();
catalog.serialize(&mut extractor).unwrap();
// extractor.names: { 0x7fff1230 => "a", 0x7fff1248 => "b", ... }

// Build pipeline — uses the same catalog instance
let pipe = construct_pipe1(&params, &catalog);

// node.input_dataset_ids() returns [0x7fff1230] → look up "a"
```

### Why It Works

Both paths reference the same struct instance:
- Pipeline nodes store `&catalog.a` → `ptr_to_id` gives address of `catalog.a`
- Serde's derive passes `&self.a` → same address (since `self` is `&catalog`)

### Considerations

- **Must use the same catalog instance** for extraction and pipeline construction. Re-extract if you create a new catalog.
- **Nested catalogs** work via recursion — the extractor updates its prefix when entering a nested struct, producing names like `"iris.input"`.
- **Parameters work too** — the `Parameters` struct also derives `Serialize`, so `"initial_value"` maps to its pointer ID.
- **The `Serializer` trait is verbose** — ~30 methods need stubbing. Use `serde_ignored` as reference for a no-op serializer implementation.
- **`MemoryDataset` serialization is trivial** — the `value` field is `#[serde(skip)]`, so it serializes to `{}`. Fine — we never use the serialized output, just the pointer addresses.

---

## Dataset Visualization (HTML)

### Approach

Add a simple optional `to_html` method to the `Dataset` trait with a default no-op:

```rust
pub trait Dataset {
    type LoadItem;
    type SaveItem;
    fn load(&self) -> Option<Self::LoadItem>;
    fn save(&self, output: Self::SaveItem);
    fn to_html(&self) -> Option<String> { None }
}
```

The viz server calls `to_html()` and renders whatever comes back. Dataset authors who care about visualization implement it. Those who don't get a generic "no preview available" placeholder in the UI.

This could alternatively be a separate `DatasetPreview` trait to keep `Dataset` focused, but adding it directly with a default is simpler and avoids a second trait.

---

## Interactive REPL / Data Exploration

### The Problem

Kedro has IPython REPL integration (`catalog.load("name")` returns a pandas DataFrame). Rust has no equivalent REPL. Datasets are extensible — users add their own types for arbitrary formats (images, audio, model weights, graphs). Can't assume tabular data. Can't hardcode renderers for each type.

### The Solution: Companion Python Packages

The dataset crate author ships a companion Python package alongside their Rust crate:

- Publish `pondrs-s3-parquet` on **crates.io** (Rust)
- Publish `pondrs-s3-parquet` on **PyPI** (Python)

Both know how to talk to the same underlying storage. The Python side returns native Python objects (polars DataFrame, PIL Image, whatever makes sense). No Rust-Python bridge — just two implementations that read the same thing.

### Discovery via Entry Points

A top-level `pondrs-catalog-py` helper auto-discovers installed companion packages using Python's `entry_points` plugin mechanism:

```python
from pondrs_catalog import load_catalog

catalog = load_catalog("catalog.yml")
df = catalog.load("iris.input")      # dispatches to pondrs-polars-py → polars DataFrame
img = catalog.load("photos.raw")     # dispatches to pondrs-image-py → PIL Image
```

Each companion package registers itself under a `pondrs.datasets` entry point group, declaring which dataset types it handles.

### Layered Approach

1. **Free (now):** The catalog YAML already describes file-based datasets with paths. Users can `polars.read_parquet(path)` directly.
2. **With viz server:** The server provides dataset HTML previews via `to_html()` and basic data exploration in the web UI (table views, stats, filtering).
3. **With companion packages:** `pondrs-catalog-py` provides `catalog.load("name")` returning native Python objects, dispatching to the right companion package.

---

## Hook System Expansion

Current hooks: `before_node_run`, `after_node_run`. For visualization, additional hooks are needed (similar to Kedro):

| Hook | Purpose |
|------|---------|
| `before_pipeline_run` | Pipeline execution starting (overall + sub-pipelines) |
| `after_pipeline_run` | Pipeline execution complete |
| `on_pipeline_error` | Pipeline-level error |
| `before_node_run` | *(exists)* Node about to execute |
| `after_node_run` | *(exists)* Node finished successfully |
| `on_node_error` | Node failed with error |
| `before_dataset_load` | About to load a dataset |
| `after_dataset_load` | Dataset loaded (could include size/shape metadata) |
| `before_dataset_save` | About to save a dataset |
| `after_dataset_save` | Dataset saved |

### VizHook Implementation

Each hook method serializes a status event to JSON and sends it over WebSocket:

```rust
struct VizEvent {
    timestamp: Instant,
    event_type: EventType,  // NodeStart, NodeEnd, NodeError, etc.
    node_name: String,
    dataset_ids: Vec<usize>,
    duration: Option<Duration>,  // for after_* events
    metadata: Option<String>,    // error message, dataset shape, etc.
}
```

### Parallel Runner

The parallel runner currently doesn't call hooks — only the sequential runner does. Hooks must be added. Thread safety consideration: use a channel-based approach where each thread sends events to a channel and a single writer drains it to the WebSocket.

---

## Libraries

### Backend (Rust)

| Library | Purpose |
|---------|---------|
| **axum** | Web framework — modern, tokio-native, built-in WebSocket support |
| **tokio** | Async runtime (required by axum) |
| **tower-http** | Middleware — CORS, static file serving, compression |
| **rust-embed** | Embed compiled frontend assets into the binary |
| **clap** (derive) | CLI parsing for `run` and `viz` commands |
| **serde_json** | JSON serialization for API responses and WebSocket messages |
| **tokio-tungstenite** | WebSocket client (for VizHook connecting to viz server) |

### Frontend (TypeScript)

| Library | Purpose |
|---------|---------|
| **React 19** | UI framework |
| **Vite** | Build tool with HMR |
| **@xyflow/react** (React Flow v12) | DAG visualization — custom nodes, edge styling, sub-flow grouping, minimap, zoom/pan |
| **shadcn/ui** | Minimal component library — copy-pasted Radix UI + Tailwind components |
| **Tailwind CSS v4** | Utility-first styling |
| **elkjs** | Graph layout engine — hierarchical/layered layouts, superior to dagre for pipeline DAGs with nesting |

---

## Interface Design

### Layout

```
┌──────────────┬───────────────────────────────────┬──────────────┐
│              │            Toolbar                 │              │
│   Catalog    │  [Run >] [Stop] [Reset]            │  Node Detail │
│   Browser    │  Pipeline: [pipe1 v]               │  Panel       │
│              ├───────────────────────────────────┤              │
│  ┌─────────┐ │                                   │  Name: ...   │
│  │ Datasets│ │      Pipeline Graph View          │  Status: ... │
│  │  ├ a    │ │                                   │  Duration: . │
│  │  ├ b    │ │   [param]──>[node1]──>[node2]     │  Inputs: ... │
│  │  ├ c    │ │                    └──>[node3]     │  Outputs: .. │
│  │  └ d    │ │                                   │              │
│  ├─────────┤ │                                   │  Dataset     │
│  │ Params  │ │                                   │  Preview     │
│  │  └ val  │ │                                   │  ┌─────────┐ │
│  └─────────┘ │                                   │  │ (html)  │ │
│              │                                   │  │         │ │
│              ├───────────────────────────────────┤  └─────────┘ │
│              │      Execution Timeline           │              │
│              │  node1 ========-------- 1.2s      │              │
│              │  node2 --------======== 0.8s      │              │
│              │  node3 --------====---- 0.4s      │              │
└──────────────┴───────────────────────────────────┴──────────────┘
```

### Pipeline Graph View (Center)

- **Nodes** as rounded rectangles with name, status indicator, elapsed time when running
- **Nested pipelines** as group nodes (React Flow parent/child grouping) — maps directly to `Pipeline` containing `Steps`
- **Datasets as edge labels** or small diamond-shaped intermediate nodes (toggleable)
- **Auto-layout via ELK** with manual drag override, layout persisted in local storage
- **Minimap** (React Flow built-in) for large pipelines

### Node Visual States

| State | Visual |
|-------|--------|
| **Idle** | Gray background, muted border |
| **Pending** | Gray background, subtle pulse |
| **Running** | Blue background, animated border |
| **Completed** | Green background, checkmark |
| **Failed** | Red background, X icon |
| **Skipped** | Dashed outline |

### Execution Timeline (Bottom)

Gantt chart of node execution over time. Especially valuable for the parallel runner — shows concurrent execution, bottlenecks, scheduling gaps. Bars colored by status, clickable to select nodes.

### Catalog Browser (Left Sidebar)

Tree view of all datasets, grouped by type. Clicking a dataset shows its `to_html()` preview in the detail panel.

### Node Detail Panel (Right Sidebar)

Selected node info: name, status, duration, inputs, outputs. Below that, the dataset preview (HTML rendered from `to_html()`).

### Theme

Dark theme by default (developer tool aesthetic). Status colors: blue (running), green (success), red (failure), amber (pending). Clean monospace typography for data, sans-serif for UI.

### Future Ideas

- **Run history** — store past execution timings, show trends
- **Partial runs** — "run from here" on a selected node
- **Filter/highlight** — type a dataset name, highlight all nodes that touch it
- **Diff view** — compare two execution runs side by side
- **Pipeline selector** — dropdown to switch between registered pipelines

---

## CLI Structure

```
pondrs run [--pipeline <name>] [--parallel] [--viz-url <url>]
pondrs viz [--port <port>] [--pipeline <name>]
```

- `--pipeline` selects which pipeline to run/visualize (from a registry)
- `--viz-url` tells the run command where to send status updates
- `--parallel` selects the parallel runner

---

## Key Decisions Still Needed

1. **Dataset naming** — implement the serde extractor, or start with explicit name fields and migrate later?
2. **Pipeline registry** — how does `viz`/`run` discover defined pipelines? A function returning named pipeline items? A static registry macro?
3. **State persistence** — should the viz server persist execution history (SQLite? JSON?) or is it purely ephemeral?
4. **WebSocket protocol** — simple JSON events or something more structured?
5. **`to_html` placement** — directly on `Dataset` trait with default, or separate `DatasetPreview` trait?
