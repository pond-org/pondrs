# Dataset Hooks

Dataset hooks fire during the load and save operations inside `Node::call()`. Each hook receives the owning node and the dataset reference.

## Methods

```rust,ignore
fn before_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
fn after_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
fn before_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
fn after_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef) {}
```

## Arguments

- **`n`** — the node that is loading/saving the dataset. Use `n.name()` to get the node name.
- **`ds`** — the dataset reference:
  - `ds.id` — unique pointer-based identifier
  - `ds.name` — resolved name from the catalog (e.g. `Some("readings")`), or `None` in `no_std`
  - `ds.meta.is_param()` — whether this is a parameter dataset
  - `ds.meta.type_string()` — the Rust type name (e.g. `"pondrs::datasets::memory::MemoryDataset<f64>"`)
  - `ds.meta.html()` — (`std` only) returns an optional HTML snippet for the dataset's current contents. Datasets like `PlotlyDataset` override this to produce rich visualizations; file-backed datasets render their contents as formatted text. Used by the viz dashboard.
  - `ds.meta.yaml()` — (`std` only) returns the dataset's configuration serialized as YAML. This is produced automatically via the `Serialize` supertrait and is used by the viz dashboard to display dataset settings.

## Firing order

For a node with two inputs and one output, hooks fire in this order:

1. `before_dataset_loaded` (input 0)
2. `after_dataset_loaded` (input 0)
3. `before_dataset_loaded` (input 1)
4. `after_dataset_loaded` (input 1)
5. *node function executes*
6. `before_dataset_saved` (output 0)
7. `after_dataset_saved` (output 0)

Dataset hooks fire **inside** the `before_node_run` / `after_node_run` window. The sequence is always: `before_node_run` → dataset loads → function call → dataset saves → `after_node_run`.

## Example: tracking I/O time

```rust,ignore
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::Instant;

struct IoTimingHook {
    starts: Mutex<HashMap<usize, Instant>>,
}

impl Hook for IoTimingHook {
    fn before_dataset_loaded(&self, _n: &dyn StepInfo, ds: &DatasetRef) {
        self.starts.lock().unwrap().insert(ds.id, Instant::now());
    }

    fn after_dataset_loaded(&self, _n: &dyn StepInfo, ds: &DatasetRef) {
        if let Some(start) = self.starts.lock().unwrap().remove(&ds.id) {
            let name = ds.name.unwrap_or("<unknown>");
            println!("  loaded {} in {:.1}ms", name, start.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
```

## Name resolution

In `std` builds, the runner resolves dataset names from the catalog indexer before dispatching to hooks. This is why `ds.name` is `Option<&str>` — it's `Some("readings")` when the catalog indexer can map the pointer to a field name, and `None` in `no_std` builds where the indexer isn't available.
