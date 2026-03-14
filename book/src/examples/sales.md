# Sales Pipeline

Monthly sales CSV processing: filter by minimum sales, compute totals,
and produce a Plotly bar chart.

## Usage

```sh
cargo run --example sales_app -- run
cargo run --example sales_app -- check
cargo run --example sales_app -- viz
```

## Types

```rust,ignore
{{#include ../../../examples/sales/mod.rs:types}}
```

## Node functions

```rust,ignore
{{#include ../../../examples/sales/mod.rs:nodes}}
```

## Pipeline definition

```rust,ignore
{{#include ../../../examples/sales/mod.rs:pipeline}}
```

## App entry point

```rust,ignore
{{#include ../../../examples/sales_app.rs:app}}
```

## Pipeline visualization

<a href="../assets/sales_viz.html" target="_blank">Open fullscreen</a>

<iframe src="../assets/sales_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
