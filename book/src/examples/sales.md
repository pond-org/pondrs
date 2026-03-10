# Sales Pipeline

Monthly sales CSV processing: filter by minimum sales, compute totals,
and produce a Plotly bar chart.

## Usage

```sh
cargo run --example sales_app -- run
cargo run --example sales_app -- check
cargo run --example sales_app -- viz
```

## Pipeline definition

```rust,no_run
{{#include ../../../examples/sales/mod.rs}}
```

## App entry point

```rust,no_run
{{#include ../../../examples/sales_app.rs}}
```

## Pipeline visualization

<iframe src="../assets/sales_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
