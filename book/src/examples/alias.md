# Alias Pipeline

Demonstrates the Alias node: write CSV as plain text, read it back
as a Polars DataFrame via Alias, and produce a Plotly bar chart.

## Usage

```sh
cargo run --example alias_app -- \
    --catalog-path examples/alias_data/catalog.yml \
    --params-path examples/alias_data/params.yml run
```

## Types

```rust,ignore
{{#include ../../../examples/alias_app.rs:types}}
```

## Node functions

```rust,ignore
{{#include ../../../examples/alias_app.rs:nodes}}
```

## Pipeline definition

```rust,ignore
{{#include ../../../examples/alias_app.rs:pipeline}}
```

## Pipeline visualization

<a href="../assets/alias_viz.html" target="_blank">Open fullscreen</a>

<iframe src="../assets/alias_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
