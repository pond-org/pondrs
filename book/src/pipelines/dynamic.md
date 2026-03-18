# Dynamic Pipelines

Sometimes the set of steps in a pipeline isn't known at compile time. A config flag might enable or disable a step, or the number of steps might depend on runtime data. `StepVec` provides a type-erased, heap-allocated step container for these cases.

## `StepVec`

```rust,ignore
pub type StepVec<'a, E = PondError> = Vec<Box<dyn RunnableStep<E> + Send + Sync + 'a>>;
```

It implements `StepInfo` and `Steps<E>`, so it works everywhere tuples do — as the return type of a pipeline function, as the `steps` field of a `Pipeline`, and with `check()`, runners, and visualization.

Use `RunnableStep::boxed()` to convert a `Node` or `Pipeline` into a boxed trait object:

```rust,ignore
let step: Box<dyn RunnableStep<PondError> + Send + Sync + 'a> =
    Node { name: "n", func: |v| (v,), input: (&a,), output: (&b,) }.boxed();
```

## Conditional nodes

The primary use case is including or excluding nodes based on runtime configuration:

```rust,ignore
{{#include ../../../examples/dyn_steps/mod.rs:pipeline}}
```

The pipeline function returns `StepVec<'a>` instead of `impl Steps<PondError> + 'a`. Each node is `.boxed()` before being added to the vec, and conditional nodes are pushed with `if`.

## Nesting inside static pipelines

`StepVec` can be used as the `steps` of a `Pipeline`, which can itself be placed in a static tuple:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    let dynamic_section = Pipeline {
        name: "optional_reports",
        steps: {
            let mut s: StepVec<'a> = vec![
                Node { name: "base_report", ... }.boxed(),
            ];
            if params.detailed.0 {
                s.push(Node { name: "detailed_report", ... }.boxed());
            }
            s
        },
        input: (&cat.summary,),
        output: (&cat.report,),
    };

    (
        Node { name: "summarize", ... },
        dynamic_section,
    )
}
```

This lets you keep type safety for the fixed parts of your pipeline and only pay for dynamic dispatch where you need it.

## Validation

`check()` works identically for `StepVec` and tuple-based pipelines — it iterates the items and validates sequential ordering, duplicate outputs, and pipeline contracts. Since `StepVec` is built at runtime, validation applies to the *constructed* pipeline only, not hypothetical alternatives. An excluded conditional node won't be checked.

If a `StepVec` is wrapped in a `Pipeline` with declared outputs, those outputs must be produced by the nodes that are actually present. An empty `StepVec` in a `Pipeline` that declares outputs will correctly fail with `UnproducedPipelineOutput`.

## When to use `StepVec` vs tuples

Use **tuples** (the default) when all nodes are known at compile time. You get zero-cost dispatch and full type checking.

Use **`StepVec`** when you need:
- Conditional inclusion/exclusion of nodes based on config or params
- A variable number of nodes determined at runtime
- Nodes of heterogeneous types that can't be expressed in a single tuple
