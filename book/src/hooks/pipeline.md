# Pipeline Hooks

Pipeline hooks fire at the boundaries of `Pipeline` structs (not flat tuples). They are useful for tracking the lifecycle of logical groups of nodes.

## Methods

```rust,ignore
fn before_pipeline_run(&self, p: &dyn StepInfo) {}
fn after_pipeline_run(&self, p: &dyn StepInfo) {}
fn on_pipeline_error(&self, p: &dyn StepInfo, error: &str) {}
```

## Arguments

- **`p`** — the pipeline being executed. `p.name()` returns the pipeline's name, `p.is_leaf()` returns `false`.
- **`error`** — the stringified error message from the failing node within the pipeline.

## Sequential runner behavior

The `SequentialRunner` fires pipeline hooks as it enters and exits `Pipeline` structs:

```text
before_pipeline_run("processing")
  before_node_run("clean")
  after_node_run("clean")
  before_node_run("transform")
  after_node_run("transform")
after_pipeline_run("processing")
```

If a child node fails, `on_pipeline_error` fires instead of `after_pipeline_run`.

## Parallel runner behavior

The `ParallelRunner` fires pipeline hooks based on dataset availability:

- `before_pipeline_run` fires when all of the pipeline's **declared inputs** are available
- `after_pipeline_run` fires when all of the pipeline's **declared outputs** have been produced
- `on_pipeline_error` fires when a child node fails — it propagates up through all ancestor pipelines

This means pipeline hooks may fire at different times than in sequential execution, since nodes may run in a different order.

## Flat tuples vs Pipeline structs

Pipeline hooks only fire for `Pipeline` structs. A flat tuple of nodes at the top level does **not** trigger `before_pipeline_run` / `after_pipeline_run`. If you want pipeline-level hooks for your entire pipeline, wrap the top-level steps in a `Pipeline`.

## Example: timing pipelines

```rust,ignore
struct PipelineTimer {
    timings: Mutex<HashMap<&'static str, Instant>>,
}

impl Hook for PipelineTimer {
    fn before_pipeline_run(&self, p: &dyn StepInfo) {
        self.timings.lock().unwrap().insert(p.name(), Instant::now());
    }

    fn after_pipeline_run(&self, p: &dyn StepInfo) {
        if let Some(start) = self.timings.lock().unwrap().remove(p.name()) {
            println!("[{}] completed in {:.1}ms", p.name(), start.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
```
