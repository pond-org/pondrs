# Recurrent Pipeline

Demonstrates `RecurrentNode`: gradient descent on `(w - target)²`, unrolled into
one node per step. Each step writes its own checkpoint file, and a final
`report` node reads the last one.

## Usage

```sh
cargo run --example recurrent_app -- run
cargo run --example recurrent_app -- run --from-nodes train/3
cargo run --example recurrent_app -- run --catalog checkpoints.count=20
cargo run --example recurrent_app -- check
cargo run --example recurrent_app -- viz
```

## Types

```rust,ignore
{{#include ../../../examples/recurrent/mod.rs:types}}
```

`checkpoints` is a `Vec<TextDataset>` written as a count and a template:

```yaml
checkpoints:
  count: 5
  template:
    path: "ckpt/epoch_{i}.txt"
```

## Node function

```rust,ignore
{{#include ../../../examples/recurrent/mod.rs:nodes}}
```

## Pipeline

```rust,ignore
{{#include ../../../examples/recurrent/mod.rs:pipeline}}
```

## App

```rust,ignore
{{#include ../../../examples/recurrent_app.rs:app}}
```

See [Recurrent Nodes](../pipelines/recurrent.md) for how unrolling works.
