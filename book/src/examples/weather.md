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

## Pipeline definition

```rust,no_run
{{#include ../../../examples/weather/mod.rs}}
```

## App entry point

```rust,no_run
{{#include ../../../examples/weather_app.rs}}
```

## Pipeline visualization

<iframe src="../assets/weather_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
