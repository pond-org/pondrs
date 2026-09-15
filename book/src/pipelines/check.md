# Check

Pipeline validation catches structural errors before execution. The `check()` method on `StepsMeta` walks the pipeline and verifies several invariants.

## Usage

```rust,ignore
let steps = pipeline(&catalog, &params);
steps.check()?;  // Result<(), CheckError>
```

Or via the CLI:

```sh
$ my_app check
Pipeline is valid.
```

## What check validates

### Sequential ordering

A node must not read a dataset that is only produced by a **later** node. Datasets that no node produces are treated as external inputs (valid).

```rust,ignore
// Valid: n1 produces a, n2 reads a
(
    Node { name: "n1", input: (&param,), output: (&a,), .. },
    Node { name: "n2", input: (&a,),     output: (&b,), .. },
)

// Invalid: n1 reads b, but b is produced by n2
(
    Node { name: "n1", input: (&b,),     output: (&a,), .. },
    Node { name: "n2", input: (&param,), output: (&b,), .. },
)
// → CheckError::InputNotProduced { node_name: "n1", .. }
```

### No duplicate outputs

A dataset must not be produced by more than one node:

```rust,ignore
(
    Node { name: "n1", input: (&param,), output: (&a,), .. },
    Node { name: "n2", input: (&param,), output: (&a,), .. },  // same output!
)
// → CheckError::DuplicateOutput { node_name: "n2", .. }
```

### Params are read-only

No node may write to a `Param` dataset. A `&Param` used directly as an output is
rejected by the compiler — `Param::SaveItem` is the uninhabited `Never`, so no
function can produce a value to save:

```rust,ignore
(
    Node { name: "n1", func: || ((),), input: (), output: (&param,) },
)
// → error[E0271]: expected tuple `((),)`, found tuple `(Never,)`
```

`check()` still catches params reached indirectly, e.g. through an `EachField`
fan-out over a catalog whose entries contain a `Param`:

```rust,ignore
// → CheckError::ParamWritten { node_name: "n1", .. }
```

### Dataset identity

Datasets are identified by the **address** of their catalog field. Two datasets
of different types that land at the same address would be merged into a single
node in the DAG, so `check()` reports them before anything else runs — an alias
also causes spurious `DuplicateOutput` / `InputNotProduced` errors further down,
and seeing those first sends you chasing a phantom.

The usual cause is a zero-sized dataset type: a field of size 0 shares its
address with whatever follows it.

```rust,ignore
struct Catalog {
    one: ZstOneDataset,   // size 0
    two: ZstTwoDataset,   // size 0 — same address as `one`
}
// → CheckError::AliasedDatasets { node_name: "n2", .. }
```

### Pipeline contracts

For `Pipeline` structs, the declared inputs and outputs must match what the children actually consume and produce. See [Pipeline](./pipeline.md).

## CheckError variants

| Variant | Meaning |
|---------|---------|
| `InputNotProduced` | Node reads a dataset produced by a later node |
| `DuplicateOutput` | Two nodes produce the same dataset |
| `ParamWritten` | A node writes to a `Param` |
| `UnusedPipelineInput` | Pipeline declares an input its children don't consume |
| `UnproducedPipelineOutput` | Pipeline declares an output its children don't produce |
| `UndeclaredPipelineInput` | A child consumes an external dataset the pipeline doesn't declare |
| `AliasedDatasets` | Two datasets of different types share one address |
| `CapacityExceeded` | Internal dataset buffer overflow (see below) |

## Warnings

Some catalogs run correctly but cannot be *named* reliably: the names that reach
logs, hooks and the viz UI end up wrong or missing. These are reported as
warnings, never errors — `check` prints them and still exits 0.

```sh
$ my_app check
warning: dataset type `myapp::S3Store` used by node 'upload' does not end in `Dataset`; the catalog indexer will recurse into it and resolve an interior field name for dataset 0x7ffd… in logs and viz
warning: dataset 0x7ffd… of type `myapp::S3Store` used by node 'upload' resolves to 's3.bucket' instead of 's3'; the type ident does not end in `Dataset`, so the catalog indexer recursed into it and an interior field name won
Pipeline is valid.
2 warnings emitted.
```

### The `*Dataset` naming convention

In Rust a struct and its first field share an address. The catalog indexer walks
the catalog with a serde `Serializer` to build its `address -> name` map, and it
stops recursing at dataset boundaries so a dataset's own name wins over its first
field's. It recognises a boundary **by the type name**:

- a type whose ident ends in `Dataset` → a leaf, not recursed into
- the type `Param` → a leaf
- anything else → a container, recursed into

So a custom dataset type **must** end in `Dataset`, and a catalog or param-group
struct **must not**. A type named `S3Store` is treated as a container: the
indexer descends into it and its first field's name (`s3.bucket`) overwrites the
dataset's own (`s3`). See [Custom Datasets](../datasets/custom_datasets.md).

### `CheckWarning` — from the pipeline alone (`no_std`)

`StepsMeta::for_each_warning` needs only the pipeline walk, so it works without
an allocator:

```rust,ignore
steps.for_each_warning(&mut |w| println!("warning: {w}"));
```

| Variant | Meaning | Fix |
|---------|---------|-----|
| `ZeroSizedDataset` | The dataset type is zero-sized, so its address is not a reliable identity | Give the type a field (a path, an id, a `PhantomData` is not enough — it is also zero-sized) |
| `UnconventionalDatasetType` | The type ident does not end in `Dataset` (and is not `Param`) | Rename the type to end in `Dataset` |

`UnconventionalDatasetType` fires under `no_std` too, where there is no indexer:
the convention is a property of the type, and the same catalog is typically also
built for host tooling and viz.

There is no sink to print to under `no_std`, so the `check` subcommand does not
report warnings there. Call `for_each_warning` yourself with your own sink (RTT,
semihosting, defmt) — see [no_std App & Debugging](../no_std/app.md).

### `CatalogWarning` — cross-checking against the catalog (`std`)

`check_catalog` compares the datasets the pipeline uses against the names the
catalog walk actually resolves for them. It needs the catalog, so it is `std`
only. Give it the **unfiltered** pipeline: a node filter (`--nodes`,
`--from-nodes`, `--to-nodes`) hides datasets and would silently narrow the check.

```rust,ignore
pondrs::check_catalog(&steps, &catalog, &params, &mut |w| println!("warning: {w}"));
```

| Variant | Meaning | Fix |
|---------|---------|-----|
| `UnnamedDataset` | The catalog walk never reached this dataset, so it has no name at all | Remove a `#[serde(skip)]`, move the dataset out of a `Vec`/tuple (the indexer does not descend into sequences), pass `&self.field` through a hand-written `Serialize`, or put the dataset in the catalog |
| `MisresolvedName` | The walk reached the dataset, but a deeper entry at the same address won, so it resolves to an interior field name | Rename the type to end in `Dataset` |

`MisresolvedName` is deliberately conservative: it fires only when an entry can
be *positively* identified as the dataset itself, by matching the serde struct
name against the pipeline's real type. A dataset with a hand-written `Serialize`
that emits no struct name (such as `CellDataset`) is left alone rather than
guessed at.

These warnings detect; they do not repair. Resolved names are unchanged — fixing
one means renaming the type or the catalog field.

## Capacity

`check()` uses a fixed-capacity buffer (default 20 datasets) for `no_std` compatibility. If your pipeline has more than 20 unique datasets, use `check_with_capacity`:

```rust,ignore
steps.check_with_capacity::<64>()?;
```

## no_std compatibility

`check()` and `for_each_warning()` both work in `no_std` environments. They use
no allocation — all dataset tracking is done in fixed-size arrays on the stack,
and warnings are handed to a callback rather than collected into a `Vec`.

`for_each_warning_with_capacity::<N>` sets the dedup capacity (default 20).
Exceeding it is not an error: deduplication simply stops, and a dataset may be
reported more than once. Dropping a warning would be worse than repeating one.

`check_catalog` is `std` only and is *not* available in `no_std`.
