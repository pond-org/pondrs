# Weather Pipeline

Demonstrates subpipelines, struct params, nested catalog/params,
PartitionedDataset, MemoryDataset, YamlDataset, PlotlyDataset,
parallel execution, and an intentional error node.

## Usage

```sh
cargo run --example weather_app -- run --runner parallel
cargo run --example weather_app -- check
cargo run --example weather_app -- viz
```

## Types

```rust,ignore
{{#include ../../../examples/weather/mod.rs:types}}
```

## Node functions

```rust,ignore
{{#include ../../../examples/weather/mod.rs:nodes}}
```

## Pipeline definition

```rust,ignore
{{#include ../../../examples/weather/mod.rs:pipeline}}
```

## App entry point

```rust,ignore
{{#include ../../../examples/weather_app.rs:app}}
```

## Pipeline visualization

<a href="../assets/weather_viz.html" target="_blank">Open fullscreen</a>

<iframe src="../assets/weather_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
