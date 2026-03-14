# Nodes

A `Node` is a single computation unit in the pipeline. It connects a function to its input and output datasets. For a deeper look at `NodeInput`/`NodeOutput` and other details, see [Pipelines & Nodes — Nodes](../pipelines/nodes.md).

## Definition

```rust,ignore
pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: StableFn<Input::Args>,
    F::Output: CompatibleOutput<Output::Output>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}
```

- **`name`** — a human-readable label used in logging, hooks, and visualization.
- **`func`** — the function to execute. Can be a closure or a named function.
- **`input`** — a tuple of dataset references to load before calling `func`.
- **`output`** — a tuple of dataset references to save the return value to.

## In the minimal example

The "summarize" node reads a CSV file, computes the mean, and stores it in memory:

```rust,ignore
{{#include ../../../examples/minimal.rs:summarize_node}}
```

The "report" node reads the mean and a parameter, then writes a JSON report:

```rust,ignore
{{#include ../../../examples/minimal.rs:report_node}}
```

## Input and output tuples

Inputs and outputs are tuples of dataset references. The function's arguments must match the `LoadItem` types of the input datasets, and its return value must match the `SaveItem` types of the output datasets.

```rust,ignore
// Single input, single output
input: (&cat.readings,),
output: (&cat.summary,),

// Multiple inputs, single output
input: (&cat.summary, &params.threshold),
output: (&cat.report,),

// No inputs (side-effect node)
input: (),
output: (&cat.result,),
```

Tuples of up to 10 elements are supported for both inputs and outputs.

## Return values

Node functions return tuples matching the output datasets. A single-output node returns a 1-tuple:

```rust,ignore
func: |x: i32| (x * 2,),  // note the trailing comma
```

Multi-output nodes return larger tuples:

```rust,ignore
func: |x: i32| (x + 1, x * 2),
output: (&cat.incremented, &cat.doubled),
```

## Fallible nodes

Node functions can return `Result` to signal errors:

```rust,ignore
Node {
    name: "parse",
    func: |text: String| -> Result<(i32,), PondError> {
        let n = text.trim().parse::<i32>().map_err(|e| PondError::Custom(e.to_string()))?;
        Ok((n,))
    },
    input: (&cat.raw_text,),
    output: (&cat.parsed_value,),
}
```

The `CompatibleOutput` trait allows both bare tuples and `Result<tuple, E>` as return types. Type mismatches between the function return and the output datasets are caught at compile time.

See [Node Errors](../error_handling/nodes.md) for more details.

## Type safety

The `Node` struct uses compile-time checks to ensure:

1. The function's argument types match the input datasets' `LoadItem` types
2. The function's return type matches the output datasets' `SaveItem` types
3. Mismatches produce compile errors at node construction, not at runtime
