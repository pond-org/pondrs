# Split & Join

The [Dynamic Pipelines](./dynamic.md) chapter showed how `StepVec` lets you include or exclude nodes based on **params** — a boolean flag controls whether a report node runs. The pipeline shape changes, but the datasets are still fixed at compile time.

Split & Join solve a different problem: the **catalog** determines the pipeline shape. When you have multiple items of the same kind — stores, regions, sensors — you want to run the same processing for each one, with each item getting its own datasets. The number of items comes from configuration, not code.

## The pattern

A typical fan-out/fan-in pipeline looks like this:

```text
 [combined data]
       │
   ┌───┴───┐        Split
   ▼       ▼
[item A] [item B]    Per-item processing
   │       │
   └───┬───┘        Join
       ▼
  [collected results]
```

1. **Split** takes a `HashMap<String, T>` from a single dataset and distributes each value to a per-item dataset
2. **Per-item nodes** process each item independently (and can run in parallel)
3. **Join** collects a value from each per-item dataset back into a `HashMap<String, T>`

## `TemplatedCatalog`

The per-item datasets live in a `TemplatedCatalog<S>` — a collection of identically-shaped catalog structs, one per item:

```rust,ignore
#[derive(Debug, Serialize, Deserialize)]
struct StoreCatalog {
    inventory: PolarsCsvDataset,
    total_value: MemoryDataset<f64>,
}

#[derive(Serialize, Deserialize)]
struct Catalog {
    // ...
    stores: TemplatedCatalog<StoreCatalog>,
    // ...
}
```

In YAML, a `TemplatedCatalog` is defined with a template and a list of names. String values containing `{placeholder}` are expanded per entry:

```yaml
stores:
  placeholder: "store"
  template:
    inventory:
      path: "data/{store}_inventory.csv"
    total_value: {}
  names: [north, south, east]
```

This produces three `StoreCatalog` instances — `north`, `south`, `east` — each with its own file path. The `placeholder` field is optional and defaults to `"name"`.

`TemplatedCatalog` serializes as a map, so the [catalog indexer](../app/viz.md) produces meaningful names like `stores.north.inventory`.

## `Split`

`Split` is a leaf node (like `Ident`) that loads a `HashMap<String, T>` from an input dataset and saves each value to the corresponding entry in a `TemplatedCatalog`. A `field` accessor selects which dataset within each entry to write to:

```rust,ignore
Split {
    name: "split_stores",
    input: &cat.grouped,                     // MemoryDataset<HashMap<String, DataFrame>>
    catalog: &cat.stores,                     // TemplatedCatalog<StoreCatalog>
    field: |s: &StoreCatalog| &s.inventory,   // which dataset to write to
}
```

At runtime, Split validates that the HashMap keys exactly match the catalog entry names. A mismatch produces a `PondError::KeyMismatch` error.

For `check()`, Split reports the single input dataset and all per-entry field datasets as outputs — so downstream nodes that read from those datasets are correctly validated.

## `Join`

`Join` is the inverse: it loads a value from each entry's dataset and collects them into a `HashMap<String, T>`:

```rust,ignore
Join {
    name: "join_values",
    catalog: &cat.stores,                       // TemplatedCatalog<StoreCatalog>
    field: |s: &StoreCatalog| &s.total_value,   // which dataset to read from
    output: &cat.store_values,                   // MemoryDataset<HashMap<String, f64>>
}
```

For `check()`, Join reports all per-entry field datasets as inputs and the single output dataset as output.

## Building per-item nodes

Between Split and Join, you need processing nodes for each item. Since the number of items is determined by YAML config, you build these dynamically with `StepVec`:

```rust,ignore
{{#include ../../../examples/split_join/mod.rs:pipeline}}
```

Each call to `cat.stores.iter()` yields `(&str, &StoreCatalog)` pairs in name-insertion order. The per-store nodes reference datasets owned by each `StoreCatalog` entry, so they are naturally wired into the correct fan-out/fan-in structure.

## Comparison with `PartitionedDataset`

`PartitionedDataset` handles a similar concept — a directory of files keyed by name — but at the dataset level. A single node reads or writes all partitions at once as a `HashMap`. Split & Join operate at the pipeline level: they let you run separate nodes for each item, with each item having its own arbitrarily complex set of datasets.

Use `PartitionedDataset` when a single node can handle all items. Use Split & Join when each item needs its own processing sub-pipeline.

## Nested templates

`TemplatedCatalog` supports nesting. An outer template can contain an inner `TemplatedCatalog` with a different placeholder:

```yaml
regions:
  placeholder: "region"
  template:
    metrics:
      placeholder: "metric"
      template:
        raw:
          path: "data/{region}/{metric}/raw.csv"
      names: [temperature, humidity]
  names: [north, south]
```

This produces paths like `data/north/temperature/raw.csv`. The outer placeholder is substituted first, so inner templates see the expanded value.
