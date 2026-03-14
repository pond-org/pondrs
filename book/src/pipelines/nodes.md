# Nodes

This page covers the `Node` struct in more depth. For the basics, see [A minimal pipeline — Nodes](../concepts/nodes.md).

## `NodeInput` and `NodeOutput` traits

These traits handle the mechanics of loading from and saving to dataset tuples:

```rust,ignore
pub trait NodeInput {
    type Args;
    fn load_data(&self, on_event: ...) -> Result<Self::Args, PondError>;
}

pub trait NodeOutput {
    type Output;
    fn save_data(&self, output: Self::Output, on_event: ...) -> Result<(), PondError>;
}
```

They are implemented for tuples of dataset references (up to 10 elements) via macros. During execution, `load_data` fires `BeforeLoad`/`AfterLoad` events for each dataset, and `save_data` fires `BeforeSave`/`AfterSave` events — these drive the [hook system](../hooks/README.md).

## `CompatibleOutput`

The `CompatibleOutput` trait is what allows node functions to return either bare tuples or `Result<tuple, E>`:

```rust,ignore
// Bare tuple — infallible node
func: |x: i32| (x * 2,),

// Result — fallible node
func: |x: i32| -> Result<(i32,), MyError> { Ok((x * 2,)) },
```

The bound `F::Output: CompatibleOutput<Output::Output>` on the `Node` struct catches type mismatches at node construction time, before the pipeline error type `E` is known. This means you get a compile error immediately if the function's return type doesn't match the output datasets.

## `IntoNodeResult`

When a node is called at runtime, `IntoNodeResult` normalizes the function's return value into `Result<O, E>`:

- A bare tuple `O` becomes `Ok(O)`
- A `Result<O, E>` is passed through as-is

This is what allows runners to handle both fallible and infallible nodes uniformly.

## Side-effect nodes

Nodes with no outputs are useful for logging, sending notifications, or other side effects:

```rust,ignore
Node {
    name: "log_summary",
    func: |summary: f64| {
        println!("Summary: {summary}");
    },
    input: (&cat.summary,),
    output: (),
}
```

A node with `output: ()` does not save any datasets. The function's return value (unit `()`) is discarded.

Similarly, a node with `input: ()` takes no arguments and produces outputs from scratch.
