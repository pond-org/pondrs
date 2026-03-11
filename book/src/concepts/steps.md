# Steps


```rust,no_run
fn seq_pipeline<'a>(
    cat: &'a Catalog,
    params: &'a Params,
) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "multiply",
            func: |v: i32, scale: i32| (v * scale,),
            input: (&params.offset, &params.scale),
            output: (&cat.a,),
        },
        Node {
            name: "add",
            func: |a: i32, off: i32| (a + off,),
            input: (&cat.a, &params.offset),
            output: (&cat.b,),
        },
        Node {
            name: "square",
            func: |b: i32| (b * b,),
            input: (&cat.b,),
            output: (&cat.c,),
        },
    )
}
```
