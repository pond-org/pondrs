# Hooks

Hooks let you observe pipeline execution events without modifying the pipeline itself. They are used for logging, timing, visualization, and custom instrumentation.

## The `Hook` trait

```rust,ignore
pub trait Hook: Sync {
    // Pipeline lifecycle
    fn before_pipeline_run(&self, p: &dyn StepInfo) {}
    fn after_pipeline_run(&self, p: &dyn StepInfo) {}
    fn on_pipeline_error(&self, p: &dyn StepInfo, error: &str) {}

    // Node lifecycle
    fn before_node_run(&self, n: &dyn StepInfo) {}
    fn after_node_run(&self, n: &dyn StepInfo) {}
    fn on_node_error(&self, n: &dyn StepInfo, error: &str) {}

    // Dataset lifecycle (fired per-dataset during Node::call)
    fn before_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
    fn after_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
    fn before_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
    fn after_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
}
```

All methods have default no-op implementations, so you only override the ones you care about.

## The `Hooks` trait

Multiple hooks are composed as a tuple:

```rust,ignore
pub trait Hooks: Sync {
    fn for_each_hook(&self, f: &mut dyn FnMut(&dyn Hook));
}
```

Implemented for tuples of up to 10 hooks, plus `()` (no hooks):

```rust,ignore
// No hooks
App::new(catalog, params).execute(pipeline)

// One hook
App::new(catalog, params)
    .with_hooks((LoggingHook::new(),))
    .execute(pipeline)

// Multiple hooks
App::new(catalog, params)
    .with_hooks((LoggingHook::new(), my_custom_hook))
    .execute(pipeline)
```

## Hook arguments

All hook methods receive `&dyn StepInfo`, which provides:

- `name()` — the node or pipeline name (`&'static str`)
- `is_leaf()` — `true` for nodes, `false` for pipelines
- `type_string()` — the Rust type name of the underlying function
- `for_each_input()` / `for_each_output()` — iterate over dataset references

Dataset hook methods additionally receive `&DatasetRef`, which provides:

- `id` — a unique identifier (pointer-based)
- `name` — an `Option<&str>` with the resolved dataset name (from the catalog indexer, available when using `std`)
- `meta` — `&dyn DatasetMeta` with `is_param()`, `type_string()`, and (with `std`) `html()` and `yaml()`

## Sync requirement

Hooks must be `Sync` because the `ParallelRunner` calls hook methods from multiple threads. For hooks that need interior mutability (e.g. to track timing), use thread-safe types like `Mutex` or `DashMap`.

This chapter covers:

- **[Dataset Hooks](./dataset.md)** — hooks for dataset load/save events
- **[Node Hooks](./node.md)** — hooks for node execution events
- **[Pipeline Hooks](./pipeline.md)** — hooks for pipeline lifecycle events
- **[Built-in Hooks](./builtin.md)** — `LoggingHook` and `VizHook`
