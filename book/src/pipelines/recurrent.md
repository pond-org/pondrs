# Recurrent Nodes

Some computations are loops: a training run repeats an epoch, a simulation
repeats a time step, and each iteration reads what the previous one wrote.
`RecurrentNode` expresses that by **unrolling the loop into one real node per
iteration**, each writing its own dataset.

```rust,ignore
{{#include ../../../examples/recurrent/mod.rs:pipeline}}
```

`build()` turns the `RecurrentNode` into an `Unrolled` group of plain `Node`s
named `train/0`, `train/1`, … Because each iteration is an ordinary node,
everything that works on nodes works on each iteration individually:

- **Resume mid-chain:** `run --from-nodes train/47` runs iterations 47 onward,
  reading iteration 46's saved output.
- **Checkpoint-aware caching:** `CacheHook` keys each iteration separately, so an
  unchanged prefix of the chain is skipped.
- **Per-iteration progress:** node hooks (`LoggingHook`, `VizHook`, your own)
  fire once per iteration.

## The state vector is the chain

`state` holds one element per iteration, so `state.len()` *is* the iteration
count. Iteration `k` is handed `state[k - 1]` as `prev` and `state[k]` as `cur`;
iteration 0 gets `init` as its `prev`. The `input` and `output` projections turn
those into the node's input and output tuples.

An element can be a bare dataset — then the projections are close to the
identity, as above — or a sub-catalog carrying per-iteration extras:

```rust,ignore
#[derive(Serialize, Deserialize)]
struct EpochSlot {
    weights: JsonDataset,
    metrics: JsonDataset,
    lr: Param<f64>,           // a learning-rate schedule, as data
}

RecurrentNode {
    name: "train",
    state: &cat.epochs,       // Vec<EpochSlot>
    init: &cat.init_epoch,    // EpochSlot
    input: |prev, cur| (&prev.weights, &cat.features, &cur.lr),
    output: |cur| (&cur.weights, &cur.metrics),
    func: train_epoch,
}
.build()
```

With a sub-catalog, `init` is a whole `EpochSlot` of which only `weights` is
read, so `check` reports `init_epoch.lr` and `init_epoch.metrics` as
`UnusedCatalogEntry`. That warning is harmless here.

Write the fields in the order shown — `state` and `init` before the projections,
`func` last. As with [`Node`](./nodes.md), fields are type-checked in written
order, and this order keeps a signature mistake reported at the closure.
`func` is cloned once per iteration, so it must be `Clone`; a plain `fn` item or
a non-capturing closure is.

## Where the slots live

Two nodes may not write the same dataset, so every iteration needs its own. A
`Vec<S>` catalog field provides that: each element has its own address, and the
catalog indexer names them `checkpoints.0`, `checkpoints.1`, … in logs, `check`
and viz.

Listing 100 elements in YAML by hand would be tedious, so
`pondrs::datasets::expand_indexed` accepts a count and a template instead:

```rust,ignore
{{#include ../../../examples/recurrent/mod.rs:types}}
```

```yaml
checkpoints:
  count: 5
  template:
    path: "ckpt/epoch_{i}.txt"
```

Element `k` is the template with `{i}` replaced by `k` in every string value; an
optional `placeholder:` key renames `{i}` when one expanded vector is nested in
another's template. A plain YAML sequence is accepted too.

The catalog is then the single source of truth for the iteration count, and it
can be overridden from the command line like any other catalog value:

```sh
cargo run --example recurrent_app -- run --catalog checkpoints.count=20
```

## Group inputs and outputs

`Unrolled` works out its own input and output contract instead of having you
declare it. Its inputs are the datasets some iteration reads and no iteration
writes (`init` and any loop-invariant inputs), and its outputs are the datasets
some iteration writes and no iteration reads (the final state). So `check`
validates it like any `Pipeline`, and the parallel runner fires
`after_pipeline_run` once the last iteration is done.
