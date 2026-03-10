# Nodes

```rust,no_run
pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: StableFn<Input::Args>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}
```
