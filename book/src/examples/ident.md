# Identity Pipeline

Demonstrates the Ident node: write CSV as plain text, read it back
as a Polars DataFrame via Ident, and produce a Plotly bar chart.

## Usage

```sh
cargo run --example ident_app -- \
    --catalog-path examples/ident_data/catalog.yml \
    --params-path examples/ident_data/params.yml run
```

## Types

```rust,ignore
{{#include ../../../examples/ident_app.rs:types}}
```

## Node functions

```rust,ignore
{{#include ../../../examples/ident_app.rs:nodes}}
```

## Pipeline definition

```rust,ignore
{{#include ../../../examples/ident_app.rs:pipeline}}
```

## Pipeline visualization

<a href="../assets/ident_viz.html" target="_blank">Open fullscreen</a>

<iframe src="../assets/ident_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
