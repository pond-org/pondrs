# Split/Join Pipeline

Demonstrates fan-out/fan-in patterns using `TemplatedCatalog`, `Split`, `Join`,
and `StepVec`. A combined inventory CSV is split by store into per-store files,
processed independently, then joined back into a comparison report.

## Usage

```sh
cargo run --example split_join_app -- run
cargo run --example split_join_app -- check
cargo run --example split_join_app -- viz
```

## Types

```rust,ignore
{{#include ../../../examples/split_join/mod.rs:types}}
```

The `StoreCatalog` struct is the per-entry template. Each store gets its own
`inventory` CSV dataset and `total_value` memory dataset, with file paths
expanded from a YAML template:

```yaml
stores:
  placeholder: "store"
  template:
    inventory:
      path: "data/{store}_inventory.csv"
    total_value: {}
  names: [north, south, east]
```

## Node functions

```rust,ignore
{{#include ../../../examples/split_join/mod.rs:nodes}}
```

## Pipeline definition

```rust,ignore
{{#include ../../../examples/split_join/mod.rs:pipeline}}
```

The pipeline uses `StepVec` because the per-store processing nodes are built
dynamically from the `TemplatedCatalog` entries. The flow is:

1. **group_by_store** — reads the combined CSV and groups rows into a `HashMap<String, DataFrame>`
2. **split_stores** — distributes each store's DataFrame to its per-store CSV file
3. **compute_store_value** (one per store) — computes total stock value from each store's CSV
4. **join_values** — collects per-store totals back into a `HashMap<String, f64>`
5. **build_report** — produces a JSON comparison report

## App entry point

```rust,ignore
{{#include ../../../examples/split_join_app.rs:app}}
```

## Pipeline visualization

<a href="../assets/split_join_viz.html" target="_blank">Open fullscreen</a>

<iframe src="../assets/split_join_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
