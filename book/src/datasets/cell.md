# Cell Dataset

`CellDataset<T>` is a stack-friendly dataset using `Cell` for `no_std` / single-threaded pipelines.

## Definition

```rust,ignore
pub struct CellDataset<T: Copy> {
    value: Cell<Option<T>>,
}
```

- Works only with `Copy` types (e.g. `i32`, `f64`, `bool`, `u8`)
- Uses `Cell` for interior mutability — no heap allocation, no locking
- Starts empty; loading before any save returns `PondError::DatasetNotLoaded`
- `const fn new()` — can be used in `static` or `const` contexts

## Usage

```rust,ignore
let a = CellDataset::<i32>::new();
let b = CellDataset::<i32>::new();

let pipe = (
    Node { name: "n1", func: |v| (v * 2,), input: (&params.x,), output: (&a,) },
    Node { name: "n2", func: |v| (v + 1,), input: (&a,), output: (&b,) },
);
```

## Thread safety

`CellDataset` implements `Sync` via an `unsafe impl` because the `RunnableStep` trait requires `Send + Sync`. This is safe **only for single-threaded runners** like `SequentialRunner`.

**Do not use `CellDataset` with `ParallelRunner`.** Use `MemoryDataset` instead for parallel pipelines.

## no_std

`CellDataset` is the primary intermediate dataset for `no_std` environments. It requires no feature flags, no allocator, and no standard library. Combined with `Param` and the `SequentialRunner`, it forms the foundation of a `no_std` pipeline.

## Limitations

- Only works with `Copy` types — cannot hold `String`, `Vec`, `DataFrame`, etc.
- Not safe for concurrent access — single-threaded use only
- No serialization of stored values — `Serialize` impl serializes as unit `()`
