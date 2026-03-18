# Dynamic Pipeline

Demonstrates runtime pipeline composition using `StepVec`: the `report`
node is only included when the `include_report` param is `true`.

## Usage

```sh
cargo run --example dyn_steps_app -- run
cargo run --example dyn_steps_app -- check
cargo run --example dyn_steps_app -- viz
```

## Types

```rust,ignore
{{#include ../../../examples/dyn_steps/mod.rs:types}}
```

## Pipeline definition

```rust,ignore
{{#include ../../../examples/dyn_steps/mod.rs:pipeline}}
```

## App entry point

```rust,ignore
{{#include ../../../examples/dyn_steps_app.rs:app}}
```

## Pipeline visualization

<a href="../assets/dyn_steps_viz.html" target="_blank">Open fullscreen</a>

<iframe src="../assets/dyn_steps_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
