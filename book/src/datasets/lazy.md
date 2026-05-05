# Lazy Dataset

`LazyDataset` wraps any dataset to defer loading and saving until the value is actually needed. Combined with `PartitionedDataset`, it enables lazy partitioned pipelines where individual partitions are only evaluated at save time — and can be processed in parallel.

*Requires the `std` feature.*

## `LazyDataset`

```rust,ignore
pub type Lazy<T, E> = Box<dyn FnOnce() -> Result<T, E> + Send>;

pub struct LazyDataset<D> {
    pub dataset: D,
}
```

`LazyDataset<D>` changes the types exposed by the inner dataset:

| | Eager (`D`) | Lazy (`LazyDataset<D>`) |
|---|---|---|
| `LoadItem` | `T` | `Lazy<T, E>` — a closure that calls `D::load()` when invoked |
| `SaveItem` | `T` | `Lazy<T, E>` — a closure that produces the value, then saves it |

Loading returns immediately with a thunk. The actual I/O happens only when the thunk is called.

## `LazyPartitionedDataset`

```rust,ignore
pub type LazyPartitionedDataset<D> = PartitionedDataset<LazyDataset<D>>;
```

A `PartitionedDataset` whose inner dataset is lazy. Loading produces a `HashMap<String, Lazy<T, E>>` — each partition is a thunk that reads its file on demand. Saving accepts a `HashMap` of thunks; each thunk is evaluated and written to its file, in parallel when possible.

### Parallel save

`LazyDataset` overrides `FileDataset::prefer_parallel()` to return `true`. When saving inside a rayon thread pool (e.g. via `ParallelRunner`), `PartitionedDataset` distributes the partition saves across threads automatically. Each thunk — including any computation chained onto it — runs in its own rayon task.

## Using `LazyPartitionedDataset` with `Node`

With a regular `Node`, the function receives the full `HashMap` of thunks at once. You can chain transformations onto each thunk without triggering any I/O — the entire chain executes lazily at save time:

```rust,ignore
fn process(
    input: HashMap<String, Lazy<String, PondError>>,
) -> (HashMap<String, Lazy<String, PondError>>,) {
    let output = input
        .into_iter()
        .map(|(name, load_thunk)| {
            let save_thunk: Lazy<String, PondError> = Box::new(move || {
                let text = load_thunk()?;       // I/O happens here, at save time
                Ok(text.to_uppercase())
            });
            (name, save_thunk)
        })
        .collect();
    (output,)
}
```

This is the most flexible approach — you control which partitions to load, can combine partitions, or skip some entirely.

## `PartitionedNode`

`PartitionedNode` is a convenience for the common case: apply the same function to every partition. You write a function that operates on a single element, and `PartitionedNode` handles the iteration, thunk wiring, and save:

```rust,ignore
pub struct PartitionedNode<'a, F, D1, D2, T1, T2> {
    pub name: &'static str,
    pub func: F,
    pub input: &'a PartitionedDataset<D1>,
    pub output: &'a PartitionedDataset<D2>,
    pub _marker: PhantomData<(T1, T2)>,
}
```

The function signature is just `fn(T1) -> (T2,)` — no `HashMap`, no thunks:

```rust,ignore
fn uppercase(text: String) -> (String,) {
    (text.to_uppercase(),)
}

PartitionedNode {
    name: "uppercase",
    func: uppercase,
    input: &catalog.input,       // PartitionedDataset or LazyPartitionedDataset
    output: &catalog.output,
    _marker: Default::default(),
}
```

You can also use the constructor:

```rust,ignore
PartitionedNode::new("uppercase", uppercase, &catalog.input, &catalog.output)
```

### How it works

`PartitionedNode` uses the `IntoThunk` and `FromThunk` traits to bridge eager and lazy datasets transparently:

1. **Load** — loads the partitioned input as a `HashMap<String, D1::LoadItem>`
2. **Map** — for each entry, wraps the loaded item as an input `Thunk<T1>` via `IntoThunk`, applies the function inside a new output `Thunk<T2>`, then converts back via `FromThunk`
3. **Save** — saves the output `HashMap`

When both input and output are lazy, the per-partition function is captured inside the output thunk and only executed at save time — which happens in parallel if using `ParallelRunner`.

### Eager, lazy, or mixed

`PartitionedNode` works with any combination of eager and lazy input/output datasets. The thunk traits handle the conversion:

| Input | Output | Behavior |
|---|---|---|
| Eager | Eager | Each partition is loaded, processed, and saved sequentially |
| Lazy | Lazy | Processing is deferred into the output thunk; parallel save |
| Lazy | Eager | Each thunk is called immediately at save time |
| Eager | Lazy | Values are wrapped in thunks; parallel save still applies |

## Full example

```rust,ignore
use pondrs::datasets::{Lazy, LazyDataset, LazyPartitionedDataset, TextDataset};
use pondrs::error::PondError;
use pondrs::{Node, PartitionedNode, ParallelRunner, Runner};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct Catalog {
    input: LazyPartitionedDataset<TextDataset>,
    via_node: LazyPartitionedDataset<TextDataset>,
    via_pnode: LazyPartitionedDataset<TextDataset>,
}

fn uppercase_map(
    input: HashMap<String, Lazy<String, PondError>>,
) -> (HashMap<String, Lazy<String, PondError>>,) {
    let output = input.into_iter().map(|(name, thunk)| {
        let out: Lazy<String, PondError> = Box::new(move || {
            Ok(thunk()?.to_uppercase())
        });
        (name, out)
    }).collect();
    (output,)
}

fn uppercase(text: String) -> (String,) {
    (text.to_uppercase(),)
}

let pipe = (
    // Manual thunk wiring via Node
    Node {
        name: "uppercase_map",
        func: uppercase_map,
        input: (&catalog.input,),
        output: (&catalog.via_node,),
    },
    // Per-partition function via PartitionedNode
    PartitionedNode {
        name: "uppercase",
        func: uppercase,
        input: &catalog.input,
        output: &catalog.via_pnode,
        _marker: Default::default(),
    },
);

ParallelRunner::new(4)
    .run::<PondError>(&pipe, &catalog, &(), &())
    .unwrap();
```

Both nodes produce the same result. `PartitionedNode` is more concise; `Node` with manual thunk mapping is more flexible.

## When to use what

- **`PartitionedDataset<D>`** (eager) — when all partitions fit in memory and you want them loaded upfront
- **`LazyPartitionedDataset<D>`** — when partitions are large or numerous and you want deferred, parallel I/O
- **`Node` with `HashMap<String, Lazy<T, E>>`** — when you need full control: filtering, combining, or skipping partitions
- **`PartitionedNode`** — when you apply the same function to every partition and want minimal boilerplate
