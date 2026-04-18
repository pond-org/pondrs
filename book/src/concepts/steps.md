# Steps

Steps are how nodes are composed into a sequence that the runner can execute. A pipeline function returns an `impl Steps<E>`, which is implemented for tuples of nodes (and pipelines). Tuples of up to 10 elements are supported.

## In the minimal example

The pipeline function returns a tuple of two nodes. This tuple automatically implements `Steps<PondError>`:

```rust,ignore
{{#include ../../../examples/minimal.rs:pipeline}}
```

The tuple ordering defines the sequential execution order. The `SequentialRunner` executes nodes in this order; the `ParallelRunner` uses dependency analysis to run independent nodes concurrently, but still respects data dependencies.

## Composing steps

Steps are just tuples, so you compose them by adding elements:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (
        Node { name: "step1", func: |x| (x,), input: (&params.x,), output: (&cat.a,) },
        Node { name: "step2", func: |a| (a + 1,), input: (&cat.a,), output: (&cat.b,) },
        Node { name: "step3", func: |b| (b * 2,), input: (&cat.b,), output: (&cat.c,) },
    )
}
```

For grouping related nodes with declared contracts, see [Pipeline](../pipelines/pipeline.md).

## Validation

`PipelineInfo::check()` validates the pipeline structure without executing it:

- No node reads a dataset before it is produced by an earlier node
- No dataset is produced by more than one node
- Parameters are never written to

```rust,ignore
let steps = pipeline(&catalog, &params);
steps.check()?;  // returns Result<(), CheckError>
```

See [Check](../pipelines/check.md) for details.

## The pipeline function

The function that creates steps must be a **named function** with an explicit lifetime, not a closure:

```rust,ignore
// Correct: named function with tied lifetime
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (/* nodes */)
}

// Wrong: closures desugar into two independent lifetimes
let pipeline = |cat: &Catalog, params: &Params| { /* ... */ };
```

This is because the `PipelineFn` trait uses a lifetime-on-trait pattern that requires both references to share the same lifetime `'a`. Named functions with explicit `<'a>` satisfy this; closures do not.
