# Dynamic Pipelines

Sometimes the set of steps in a pipeline isn't known at compile time. A config flag might enable or disable a step, or the number of steps might depend on runtime data. `DynSteps` provides a type-erased, heap-allocated step container for these cases.

## `DynSteps`

```rust,ignore
pub type DynSteps<'a, E = PondError> = Vec<Box<dyn Step<E> + Send + Sync + 'a>>;
```

It implements `StepsMeta` and `Steps<E>`, so it works everywhere tuples do — as the return type of a pipeline function, as the `steps` field of a `Pipeline`, and with `check()`, runners, and visualization.

Use `Step::boxed()` to convert a `Node` or `Pipeline` into a boxed trait object:

```rust,ignore
let step: Box<dyn Step<PondError> + Send + Sync + 'a> =
    Node { name: "n", func: |v| (v,), input: (&a,), output: (&b,) }.boxed();
```

## Conditional nodes

The primary use case is including or excluding nodes based on runtime configuration:

```rust,ignore
{{#include ../../../examples/dyn_steps/mod.rs:pipeline}}
```

The pipeline function returns `DynSteps<'a>` instead of `impl Steps<PondError> + 'a`. Each node is `.boxed()` before being added to the vec, and conditional nodes are pushed with `if`.

## Nesting inside static pipelines

`DynSteps` can be used as the `steps` of a `Pipeline`, which can itself be placed in a static tuple:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    let dynamic_section = Pipeline {
        name: "optional_reports",
        steps: {
            let mut s: DynSteps<'a> = vec![
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

`check()` works identically for `DynSteps` and tuple-based pipelines — it iterates the items and validates sequential ordering, duplicate outputs, and pipeline contracts. Since `DynSteps` is built at runtime, validation applies to the *constructed* pipeline only, not hypothetical alternatives. An excluded conditional node won't be checked.

If a `DynSteps` is wrapped in a `Pipeline` with declared outputs, those outputs must be produced by the nodes that are actually present. An empty `DynSteps` in a `Pipeline` that declares outputs will correctly fail with `UnproducedPipelineOutput`.

## When to use `DynSteps` vs tuples

Use **tuples** (the default) when all nodes are known at compile time. You get zero-cost dispatch and full type checking.

Use **`DynSteps`** when you need:
- Conditional inclusion/exclusion of nodes based on config or params
- A variable number of nodes determined at runtime
- Nodes of heterogeneous types that can't be expressed in a single tuple
