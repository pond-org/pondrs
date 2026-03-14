# Viz

The `viz` subcommand builds the pipeline graph and serves an interactive visualization.

*Requires the `viz` feature.*

## CLI usage

```sh
# Start the interactive web server (default port 8080)
$ my_app viz

# Custom port
$ my_app viz --port 3000

# Export graph as JSON
$ my_app viz --output pipeline.json

# Export self-contained HTML file
$ my_app viz --export pipeline.html
```

## Interactive server

`my_app viz` starts an axum web server with:

| Endpoint | Description |
|----------|-------------|
| `GET /api/graph` | Full pipeline graph as JSON |
| `GET /api/dataset/{id}/html` | HTML snapshot for a dataset |
| `GET /api/status` | Current node/dataset execution status |
| `POST /api/status` | Receives live events from `VizHook` |
| `GET /ws` | WebSocket broadcast of live events |
| `GET /` | Embedded React frontend |

The frontend uses React + ReactFlow + dagre for a left-to-right DAG layout with:

- Node, dataset, and pipeline visual types
- Click-to-inspect dataset details (HTML preview, YAML config)
- Left sidebar listing all nodes, datasets, and parameters
- Dark/light theme support

## Live execution status

To see real-time node status during pipeline execution:

1. Start the viz server: `my_app viz --port 8080`
2. In another terminal, run the pipeline with `VizHook`:

```rust,ignore
use pondrs::viz::VizHook;

App::from_yaml(..)?
    .with_args(std::env::args_os())?
    .with_hooks((LoggingHook::new(), VizHook::new("http://localhost:8080".into())))
    .dispatch(pipeline)?;
```

The frontend shows live node status (idle/running/completed/error) and updates automatically via WebSocket.

## Static HTML export

`--export` produces a self-contained HTML file with the full graph and dataset snapshots embedded. No server needed — just open the file in a browser:

```sh
$ my_app viz --export pipeline.html
$ open pipeline.html
```

The export uses `vite-plugin-singlefile` to inline all JS/CSS into a single HTML file, and injects graph data via `window.__STATIC_DATA__`.

## JSON export

`--output` writes the `VizGraph` as JSON, useful for custom tooling:

```sh
$ my_app viz --output graph.json
```

## Graph name

The graph name shown in the frontend header is derived from the program name (first CLI argument, file stem only). For example, running `cargo run --example weather_app` sets the name to `weather_app`.
