# Nodes

```rust,no_run
pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: StableFn<Input::Args>,
    F::Output: CompatibleOutput<Output::Output>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}
```
```rust,no_run
fn greetings(bananas: i32) -> (String,) {
    let greeting = format!("We have %d bananas!", bananas);
    (greeting.to_string(),)
}


let node = Node {
    name: "greetings",
    func: greetings,
    input: (&params.bananas),
    output: (&catalog.greetings,),
}
```
