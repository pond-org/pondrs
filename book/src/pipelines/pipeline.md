# Pipeline

The `Pipeline` struct groups related steps into a named container with declared input/output dataset contracts.

## Definition

```rust,ignore
pub struct Pipeline<S: PipelineInfo, Input: NodeInput, Output: NodeOutput> {
    pub name: &'static str,
    pub steps: S,
    pub input: Input,
    pub output: Output,
}
```

- **`name`** — label for logging, hooks, and visualization
- **`steps`** — a tuple of nodes (and/or nested pipelines)
- **`input`** — datasets this pipeline expects to be available when it runs
- **`output`** — datasets this pipeline guarantees to produce

Pipelines are containers — runners never call them directly. Instead, they recurse into the pipeline's steps. The `StepInfo::is_leaf()` method returns `false` for pipelines, signaling the runner to descend into children.

## Example

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (
        Node { name: "load", func: |v| (v,), input: (&params.input,), output: (&cat.raw,) },
        Pipeline {
            name: "processing",
            steps: (
                Node { name: "clean", func: |v| (v,), input: (&cat.raw,), output: (&cat.clean,) },
                Node { name: "transform", func: |v| (v * 2,), input: (&cat.clean,), output: (&cat.result,) },
            ),
            input: (&cat.raw,),
            output: (&cat.result,),
        },
    )
}
```

## Input/output contracts

The `input` and `output` declarations are validated by `check()`:

- Every declared **input** must be consumed by at least one child node
- Every declared **output** must be produced by at least one child node

If these contracts are violated, `check()` returns `CheckError::UnusedPipelineInput` or `CheckError::UnproducedPipelineOutput`.

## Nesting

Pipelines can be nested arbitrarily. The validator and runners recurse through the tree:

```rust,ignore
Pipeline {
    name: "outer",
    steps: (
        Pipeline {
            name: "inner",
            steps: (/* nodes */),
            input: (/* ... */),
            output: (/* ... */),
        },
        Node { /* ... */ },
    ),
    input: (/* ... */),
    output: (/* ... */),
}
```

## Hooks and visualization

Pipeline boundaries fire their own hook events (`before_pipeline_run`, `after_pipeline_run`, `on_pipeline_error`) distinct from node events. In the visualization, pipelines appear as expandable containers that group their child nodes.

## When to use Pipeline vs flat tuples

Use a flat tuple of nodes when the pipeline is simple and linear. Use `Pipeline` when you want to:

- Name a group of related steps for logging and visualization
- Declare input/output contracts that are validated by `check()`
- Organize large pipelines into logical sections
