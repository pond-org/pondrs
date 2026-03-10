# Register Pipeline

Simulates hardware register access: reads a sensor register, processes
the value against thresholds, and sets GPIO pins accordingly.
Demonstrates RegisterDataset, GpioDataset, and Param with a
programmatically constructed catalog.

## Usage

```sh
cargo run --example register_example -- run
cargo run --example register_example -- viz
```

## Source

```rust,no_run
{{#include ../../../examples/register_example.rs}}
```

## Pipeline visualization

<iframe src="../assets/register_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
