# Parallel Runner

The `ParallelRunner` executes independent pipeline nodes concurrently using scoped threads.

*Requires the `std` feature.*

## Overview

```rust,ignore
#[derive(Default)]
pub struct ParallelRunner;

impl Runner for ParallelRunner {
    fn name(&self) -> &'static str { "parallel" }
    // ...
}
```

## How it works

1. **Builds a dependency graph** from the pipeline using `build_pipeline_graph()`
2. **Identifies source datasets** — params and external inputs that are available immediately
3. **Schedules nodes** as soon as all their input datasets have been produced
4. **Tracks produced datasets** — when a node completes, its output datasets become available, potentially unblocking other nodes
5. **Uses `std::thread::scope`** for safe scoped threads — no `'static` bounds needed

## Usage

```sh
$ my_app run --runner parallel
```

Or programmatically:

```rust,ignore
use pondrs::runners::ParallelRunner;

App::new(catalog, params)
    .with_runners((SequentialRunner, ParallelRunner))
    .execute(pipeline)?;
```

## Dependency analysis

The parallel runner determines which nodes can run concurrently by analyzing dataset dependencies:

```text
    param
    /    \
  [a]    [b]     ← a and b can run in parallel (both read param)
    \    /
     [c]         ← c waits for both a and b to complete
```

Only **data dependencies** matter — the tuple ordering in your pipeline function is irrelevant to the parallel runner.

## Error handling

On the first node failure:

1. `on_node_error` fires for the failed node
2. `on_pipeline_error` fires for all ancestor pipelines
3. No new nodes are scheduled
4. Already-running nodes are allowed to complete (drain)
5. The first error is returned

## Pipeline hooks

Pipeline hooks behave differently than in the sequential runner:

- `before_pipeline_run` fires when all of the pipeline's **declared inputs** are available
- `after_pipeline_run` fires when all of the pipeline's **declared outputs** have been produced

This means pipeline hooks may fire at different points in wall-clock time compared to sequential execution.

## Thread safety requirements

Because nodes run on different threads:

- Hooks must be `Sync` (use `Mutex` or atomics for mutable state)
- Datasets used concurrently must support concurrent access. `MemoryDataset` uses `Arc<Mutex<_>>` and is safe. `CellDataset` is **not** safe for parallel use.

## When to use

Use the parallel runner when:
- Your pipeline has independent branches that can run concurrently
- Nodes involve I/O (file reads, network calls) where parallelism helps
- You're in a `std` environment with thread support
