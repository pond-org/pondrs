# Compile Errors

Pipelines are checked by the type system, so most mistakes surface as a compiler
error rather than a failed run. This chapter maps the errors you are likely to
hit onto what they actually mean.

## Write your fields in the order `name`, `input`, `output`, `func`

Rust type-checks struct-literal fields in the order you write them. Listing
`input` and `output` before `func` means the loaded argument types and the
expected return type are already known when the compiler reaches the closure, so
a mismatch is reported against the closure — with the signature it expected —
instead of as an opaque associated-type mismatch pointing at `Node {`.

```rust,ignore
Node {
    name: "scale",
    input: (&cat.raw, &params.factor),
    output: (&cat.scaled,),
    func: |raw: Vec<f64>, factor: f64| (raw.iter().map(|v| v * factor).collect(),),
}
```

The order has no effect on behaviour. It only changes the diagnostics. With
`func` last, a wrong argument type reads:

```text
error[E0631]: type mismatch in closure arguments
   |
13 |         func: |s: String| (s.len() as i32,),
   |               ----------- found signature defined here
   |
   = note: expected closure signature `fn(i32) -> _`
              found closure signature `fn(String) -> _`
```

With `func` first, the same mistake reads:

```text
error[E0271]: type mismatch resolving `<(&Param<i32>,) as NodeInput>::Args == (String,)`
 --> src/main.rs:9:14
  |
9 |     let _n = Node {
  |              ^^^^ expected `(String,)`, found `(i32,)`
```

Both point at the same bug. Only the first tells you where to look.

## The rules a node has to satisfy

A node function takes the `LoadItem` of each input dataset, in order, and
returns a tuple of the `SaveItem` of each output dataset, in order — or a
`Result` of that tuple when it can fail. Everything below is a violation of one
of those two sentences.

### Wrong argument type

```text
error[E0631]: type mismatch in closure arguments
   = note: expected closure signature `fn(i32) -> _`
              found closure signature `fn(String) -> _`
```

The closure's parameter does not match the input dataset's `LoadItem`. Check the
dataset type in the catalog: a `Param<i32>` loads an `i32`, a `TextDataset`
loads a `String`, a `PolarsCsvDataset` loads a `DataFrame`.

### Wrong number of arguments

```text
error[E0593]: closure is expected to take 2 arguments, but it takes 1 argument
```

The closure takes one parameter per entry in the `input` tuple. An empty
`input: ()` means a closure with no parameters.

### Wrong return type

```text
error[E0277]: node function returns `(i32,)`, but `output` expects `(String,)`
   = note: a node function returns the `SaveItem` of each output dataset, in order
   = note: return `(String,)`, or `Result<(String,), E>` where the pipeline error
           type implements `From<E>`
```

### Returning a value instead of a tuple

```text
error[E0277]: node function returns `i32`, but `output` expects `(i32,)`
```

A single output is still a one-element tuple. Write `(value,)` — with the comma —
not `value`.

### Passing a dataset by value

```text
error[E0277]: `Param<i32>` cannot be used as a node input
   = note: input tuples hold dataset *references*: write `(&catalog.field,)`,
           not `(catalog.field,)`
```

### Writing to a `Param`

```text
error[E0277]: node function returns `((),)`, but `output` expects `(Never,)`
```

Params are read-only. Their `SaveItem` is the uninhabited [`Never`] type, so no
function can produce a value to save — putting a `&Param` in an `output` tuple
cannot type-check. Remove it from the output tuple.

A param reached *indirectly*, through an [`EachField`] over a catalog that
happens to contain one, is not caught this way; that case is reported by
[`check()`](./check.md) as `CheckError::ParamWritten`.

[`Never`]: ../datasets/param.md
[`EachField`]: ./split_join.md

### `func` is not callable

```text
error[E0277]: expected a `Fn(i32)` closure, found `u8`
   = note: required for `u8` to implement `StableFn<(i32,)>`
```

The `func` field holds something that is not a function or closure.

## Error-type errors

### The node's error does not convert

```text
error[E0277]: node function returns `Result<(i32,), MyErr>`, which cannot produce
              output `(i32,)` in a pipeline with error type `PondError`
   = note: return `(i32,)`, or `Result<(i32,), E2>` where `PondError` implements
           `From<E2>`
```

A fallible node may return any error type, provided the pipeline's error type
converts from it. Add `impl From<MyErr> for PondError`, or map the error inside
the node. See [Node Errors](../error_handling/nodes.md).

### The pipeline error type does not convert from `PondError`

```text
error[E0277]: the trait bound `MyErr: From<PondError>` is not satisfied
```

Every pipeline error type must absorb the library's own failures, because
loading and saving datasets produce `PondError`. Add `impl From<PondError> for
MyErr`. See [Error Type](../error_handling/error_type.md).

### A dataset's error does not convert

```text
error[E0277]: the trait bound `PondError: From<MyErr>` is not satisfied
   = note: required for `&MyDataset` to implement `DatasetInput`
```

The trailing `note` is the important line: a custom `Dataset` can only be used in
a node once `PondError: From<Self::Error>` holds. See
[Dataset Errors](../error_handling/datasets.md).

## Pipeline and hook errors

### Something in the tuple is not a step

```text
error[E0277]: `u8` is not a pipeline step
   = note: steps are `Node`, `Pipeline`, `Ident`, `PartitionedNode`, or a boxed
           step in a `StepVec`
```

### Something in the tuple is not a hook

```text
error[E0277]: `u32` is not a hook
   = note: implement `Hook` for `u32`, or wrap a `TypedHook` with `.typed()`
```

### The pipeline function is a closure

```text
error: lifetime may not live long enough
   = note: ...closure implements `Fn(&Catalog, &Params) -> ...`
```

Pipeline functions must be named functions with an explicit lifetime:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    // ...
}
```

A closure desugars into two independent lifetimes for its two reference
parameters, so the returned steps cannot borrow from both. This is a limitation
of closure inference, not of the pipeline API.

## Partitioned nodes

```text
error[E0277]: the input partition yields `String`, but the node function takes `i32`
error[E0277]: the node function returns `i32`, but the output partition stores `String`
```

A [`PartitionedNode`](../datasets/partitioned.md) maps one element of the input
partition to one element of the output partition. Because its element types are
inferred from the function's own signature, a mismatch against the datasets is
reported where the node is used as a step rather than where it is constructed.
