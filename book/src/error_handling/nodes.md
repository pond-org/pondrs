# Node Errors

Node functions can be either infallible (returning a bare tuple) or fallible (returning `Result`).

## Infallible nodes

The simplest nodes return a bare tuple. They cannot fail:

```rust,ignore
Node {
    name: "double",
    func: |x: i32| (x * 2,),
    input: (&params.x,),
    output: (&cat.doubled,),
}
```

## Fallible nodes

Nodes that can fail return `Result<(outputs...), E>` where `E` is the pipeline error type:

```rust,ignore
Node {
    name: "parse",
    func: |text: String| -> Result<(i32,), PondError> {
        let n = text.trim().parse::<i32>()
            .map_err(|e| PondError::Custom(e.to_string()))?;
        Ok((n,))
    },
    input: (&cat.raw_text,),
    output: (&cat.parsed,),
}
```

## How it works: `IntoNodeResult`

The `IntoNodeResult` trait normalizes both bare tuples and `Result` returns into `Result<O, E>`:

```rust,ignore
pub trait IntoNodeResult<O, E> {
    fn into_node_result(self) -> Result<O, E>;
}
```

- For bare tuples `O`: wraps in `Ok(value)`
- For `Result<O, E>`: passes through unchanged

This means the same `Node` struct works for both infallible and fallible functions.

## `CompatibleOutput`

The `CompatibleOutput` trait ensures at **compile time** that the function's return type matches the output datasets. It accepts both:

- `O` (bare tuple) — direct match
- `Result<O, E>` — the `Ok` type must match

If the types don't match, you get a compile error when constructing the `Node`, not at runtime.

## Error propagation

When a fallible node returns `Err(e)`:

1. The runner catches the error
2. The `on_node_error` hook fires with the error message
3. If the node is inside a `Pipeline`, `on_pipeline_error` fires for each ancestor
4. The runner stops executing and returns the error

## Using custom error types

With a custom error type, nodes can return domain-specific errors:

```rust,ignore
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error(transparent)]
    Pond(#[from] PondError),
    #[error("value {0} exceeds limit")]
    TooLarge(f64),
}

Node {
    name: "validate",
    func: |value: f64| -> Result<(f64,), MyError> {
        if value > 100.0 {
            return Err(MyError::TooLarge(value));
        }
        Ok((value,))
    },
    input: (&cat.raw,),
    output: (&cat.validated,),
}
```
