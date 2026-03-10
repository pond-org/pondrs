# Identity Pipeline

Demonstrates the Ident node: write CSV as plain text, read it back
as a Polars DataFrame via Ident, and produce a Plotly bar chart.

## Usage

```sh
cargo run --example ident_app -- \
    --catalog-path examples/ident_data/catalog.yml \
    --params-path examples/ident_data/params.yml run
```

## Source

```rust,no_run
{{#include ../../../examples/ident_app.rs}}
```

## Pipeline visualization

<iframe src="../assets/ident_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
