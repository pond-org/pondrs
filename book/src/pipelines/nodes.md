# Nodes

This page covers the `Node` struct in more depth. For the basics, see [A minimal pipeline — Nodes](../concepts/nodes.md).

## `NodeInput` and `NodeOutput` traits

These traits handle the mechanics of loading from and saving to dataset tuples. Each is split along the same axis as `StepMeta`/`Step<E>`: a non-generic metadata half, and an `E`-generic execution half.

```rust,ignore
// Metadata: what the tuple loads/saves, and which datasets it names.
pub trait NodeInputMeta: StableTuple {
    type Args: StableTuple;
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

pub trait NodeOutputMeta: StableTuple {
    type Output: StableTuple;
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

// Execution: converts each slot's error into the pipeline error type `E`.
pub trait NodeInput<E>: NodeInputMeta {
    fn load_data(&self, on_event: ...) -> Result<Self::Args, E>;
}

pub trait NodeOutput<E>: NodeOutputMeta {
    fn save_data(&self, output: Self::Output, on_event: ...) -> Result<(), E>;
}
```

They are implemented for tuples of dataset references (up to 10 elements) via macros. During execution, `load_data` fires `BeforeLoad`/`AfterLoad` events for each dataset, and `save_data` fires `BeforeSave`/`AfterSave` events — these drive the [hook system](../hooks/README.md).

The split exists because the error bound is variadic: `(T0, T1)` needs `E: From<T0::Error> + From<T1::Error>`, which only an impl header can express, and that forces `E` onto the trait. `Args` cannot come along — `Node` resolves `Input::Args` to type-check the closure *before* `E` is known, which is what gives you closure-signature diagnostics at the `Node { .. }` literal. So `Node` bounds its `Input`/`Output` on the `Meta` traits, and the `Leaf<E>`/`Step<E>` impls add the `E`-generic ones.

The practical consequence: argument- and return-type mistakes are reported at the `Node { .. }` literal, while error-conversion mistakes are reported where `E` is named. See [Compile errors](./errors.md).

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
    input: (&cat.summary,),
    output: (),
    func: |summary: f64| {
        println!("Summary: {summary}");
    },
}
```

A node with `output: ()` does not save any datasets. The function's return value (unit `()`) is discarded.

Similarly, a node with `input: ()` takes no arguments and produces outputs from scratch.
