# Runner Trait

The `Runner` trait defines how a pipeline is executed. Implementing it lets you create custom execution strategies.

## Definition

```rust,ignore
pub trait Runner {
    fn name(&self) -> &'static str;

    fn run<E>(
        &self,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + Display + Debug + 'static;
}
```

## Implementing a runner

A runner walks the pipeline steps and executes each node. The key types to work with are:

- **`Steps<E>`** — iterate steps via `for_each_item()`
- **`RunnableStep<E>`** — each step, which is either a leaf node or a pipeline container
  - `is_leaf()` — `true` for nodes, `false` for pipelines
  - `call(on_event)` — execute a leaf node, passing a callback for dataset events
  - `for_each_child_step()` — iterate children of a pipeline container
- **`Hooks`** — fire lifecycle events via `for_each_hook()`

See the `SequentialRunner` and `ParallelRunner` source code for concrete implementation examples.

## Dataset event dispatch

When calling `item.call(on_event)`, the `on_event` callback receives `(&DatasetRef, DatasetEvent)` for each dataset load/save. The built-in runners use the `dispatch_dataset_event` helper to resolve dataset names from the catalog index and forward to hooks. A custom runner can do the same or handle events differently.

## Catalog indexing

The `catalog` and `params` arguments to `run()` are provided so the runner can build a dataset name index. The built-in runners use `index_catalog_with_params(catalog, params)` (available in `std`) to create a `HashMap<usize, String>` mapping pointer IDs to field names. This is optional — a `no_std` runner can ignore these arguments.
