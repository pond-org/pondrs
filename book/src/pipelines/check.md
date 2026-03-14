# Check

Pipeline validation catches structural errors before execution. The `check()` method on `StepInfo` walks the pipeline and verifies several invariants.

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

No node may write to a `Param` dataset:

```rust,ignore
(
    Node { name: "n1", func: || ((),), input: (), output: (&param,) },
)
// → CheckError::ParamWritten { node_name: "n1", .. }
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
| `CapacityExceeded` | Internal dataset buffer overflow (see below) |

## Capacity

`check()` uses a fixed-capacity buffer (default 20 datasets) for `no_std` compatibility. If your pipeline has more than 20 unique datasets, use `check_with_capacity`:

```rust,ignore
steps.check_with_capacity::<64>()?;
```

## no_std compatibility

`check()` works in `no_std` environments. It uses no allocation — all dataset tracking is done in fixed-size arrays on the stack.
