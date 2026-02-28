# Plan: no_std Graph Validation & Node Names for Sequential Runner

## Status Quo

The crate is `#![no_std]` at the root, with `std` gated behind a feature flag. In no_std mode the `SequentialRunner` works today, but two important pieces are missing:

1. **No graph validation** — the entire `graph` module (build + validate) requires `std` for `HashMap`, `HashSet`, `Vec`, and `String`. In no_std the sequential runner just executes nodes in definition order and hopes for the best.

2. **No dataset/node names** — the `catalog_indexer` uses a custom serde `Serializer` to walk catalog structs and build a `HashMap<usize, String>` mapping pointer IDs to dotted field names (e.g. `"catalog.a"`). This is std-only. In no_std the sequential runner has access to `node.name` (a `&'static str`), but hook callbacks and error messages cannot reference dataset names.

This document evaluates approaches for both problems, aiming for minimal complexity.

---

## Part 1: no_std Graph Validation

### What needs checking

The current `PipelineGraph::check()` performs four validations:

| Check | What it does | Collections used |
|-------|-------------|-----------------|
| `check_missing_inputs` | Every non-param leaf input is produced by some leaf output | `HashSet<usize>` for produced set |
| `check_duplicate_outputs` | No dataset is written by two leaves | `HashMap<usize, Vec<&str>>` |
| `check_param_writes` | No leaf writes to a param dataset | none (simple loop) |
| `check_pipeline_contracts` | Pipeline declared I/O matches children's actual I/O | `HashSet<usize>` × 2, recursive |

The key simplification the user suggests: **assume (and verify) that nodes within each pipeline are in sequential order**, i.e. a node's non-param inputs must be produced by an earlier node (or be a pipeline input / param). This is already how the sequential runner works — it executes in definition order — so the constraint is natural.

### Approach A: Walk-and-accumulate with `heapless::FnvIndexSet`

Use bounded fixed-capacity sets from the `heapless` crate to track "produced so far" as we walk nodes in order.

```rust
use heapless::FnvIndexSet;

/// Validate that the pipeline is sequentially valid.
/// N = maximum number of distinct datasets in the pipeline.
fn validate_sequential<const N: usize>(
    pipe: &impl StepInfo,
    params: &impl StepInfo,  // or: param IDs pre-collected
) -> Result<(), ValidationError> {
    // Seed with param dataset IDs
    let mut produced: FnvIndexSet<usize, N> = FnvIndexSet::new();
    collect_param_ids(params, &mut produced);

    // Walk leaves in definition order
    pipe.for_each_info(&mut |item| {
        validate_node_recursive::<N>(item, &mut produced, &mut errors);
    });
    // ...
}

fn validate_node_recursive<const N: usize>(
    item: &dyn PipelineInfo,
    produced: &mut FnvIndexSet<usize, N>,
    errors: &mut /* ??? */,
) {
    if item.is_leaf() {
        // Check: every non-param input must already be in `produced`
        item.for_each_input_id(&mut |d| {
            if !d.is_param && !produced.contains(&d.id) {
                // ERROR: input not yet produced — violates sequential order
            }
        });
        // Check: no output is a param
        item.for_each_output_id(&mut |d| {
            if d.is_param {
                // ERROR: writing to a param
            }
            // Check: no duplicate output
            if produced.contains(&d.id) {
                // ERROR: dataset already produced by earlier node
            }
            produced.insert(d.id).ok(); // ignores capacity overflow
        });
    } else {
        // Pipeline node: check contract, then recurse into children
        // (see pipeline contract discussion below)
        item.for_each_child(&mut |child| {
            validate_node_recursive::<N>(child, produced, errors);
        });
    }
}
```

**Pros:**
- Simple single-pass algorithm: O(nodes × max_inputs)
- Fixed memory with no allocator; the const generic `N` bounds capacity
- Naturally checks sequential ordering as a side-effect
- `heapless::FnvIndexSet` is `no_std` and well-maintained

**Cons:**
- Requires the user to pick a const `N` (max distinct datasets)
- Capacity overflow is silent or panics — must be handled
- `heapless` adds a dependency (~38K lines, but it's a common embedded crate)

### Approach B: Walk-and-accumulate with a flat `[usize; N]` array

Avoid `heapless` entirely. Use a simple sorted array to track produced dataset IDs.

```rust
struct ProducedSet<const N: usize> {
    ids: [usize; N],
    len: usize,
}

impl<const N: usize> ProducedSet<N> {
    const fn new() -> Self {
        Self { ids: [0; N], len: 0 }
    }

    fn contains(&self, id: usize) -> bool {
        self.ids[..self.len].contains(&id)
    }

    fn insert(&mut self, id: usize) -> Result<(), ()> {
        if self.contains(id) { return Ok(()); }
        if self.len >= N { return Err(()); }
        self.ids[self.len] = id;
        self.len += 1;
        Ok(())
    }
}
```

**Pros:**
- Zero external dependencies
- Trivially simple — easy to audit, no hash function concerns
- `const`-constructible

**Cons:**
- `contains` is O(n) linear scan (but n is typically small — 10s of datasets, not 1000s)
- Same const generic capacity issue as Approach A
- Less ergonomic than a real set

### Approach C: Validation purely through `PipelineInfo` trait, no collections

Check _only_ the sequential ordering constraint without maintaining a produced set. Instead, for each node, walk backwards through earlier siblings to see if inputs are satisfied.

This is conceptually appealing but impractical: `for_each_info` / `for_each_child` only provides forward iteration through closures. There's no random access to "the node before me". You'd need to either:
- Collect nodes into an array first (back to Approach A/B), or
- Add a `for_each_info_indexed` method to `StepInfo`

**Verdict:** Not simpler than A or B. Discard.

### Approach D: Compile-time validation via the type system

In theory, one could encode "dataset X is produced" into the type system so that a node consuming X can only appear in a context where X has been produced. This would be a dependent-type-like encoding using generic parameters.

**Verdict:** Extremely complex, would require a complete redesign of the Pipeline/Node types, and would make the API much harder to use. Not feasible as an incremental addition.

### Recommendation for Part 1

**Approach B (flat array)** is recommended as the starting point. It adds zero dependencies, is trivially auditable, and the O(n) lookup is fine for real-world pipeline sizes (typically < 100 datasets). If profiling later shows this matters, upgrading to Approach A (heapless FnvIndexSet) is a drop-in replacement.

The validation function would:

1. Live in a new module, e.g. `src/core/validate.rs` (not gated by `std`)
2. Operate directly on `&dyn PipelineInfo` via the existing trait methods
3. Not require building a `PipelineGraph` at all
4. Return a simple enum error (no `String` formatting — use `&'static str` node names and `usize` dataset IDs)

### Pipeline contract checking in no_std

The existing `check_pipeline_contracts` verifies that:
- Every declared pipeline input is consumed by some descendant
- Every declared pipeline output is produced by some descendant

This can be done with the same `ProducedSet` approach. After recursing into all children of a pipeline, check that the pipeline's declared outputs are in `produced`, and collect a `ConsumedSet` alongside to check declared inputs.

```rust
// After recursing through pipeline children:
//   - produced: all datasets produced by descendants
//   - consumed: all datasets consumed by descendants (tracked similarly)
//
// Check: pipeline.output ⊆ produced
// Check: pipeline.input ⊆ consumed
```

This adds one more flat array of the same size, doubling the stack usage but remaining simple.

### Error reporting without String

In no_std, we can't format `String` error messages. The validation error type would use only static data:

```rust
#[derive(Debug)]
pub enum NoStdValidationError {
    /// Node requires a dataset not yet produced by any earlier node.
    InputNotProduced {
        node_name: &'static str,
        dataset_id: usize,
    },
    /// Multiple nodes produce the same dataset.
    DuplicateOutput {
        node_name: &'static str,
        dataset_id: usize,
    },
    /// A node writes to a param (read-only) dataset.
    ParamWritten {
        node_name: &'static str,
        dataset_id: usize,
    },
    /// Pipeline declares an input not consumed by children.
    UnusedPipelineInput {
        pipeline_name: &'static str,
        dataset_id: usize,
    },
    /// Pipeline declares an output not produced by children.
    UnproducedPipelineOutput {
        pipeline_name: &'static str,
        dataset_id: usize,
    },
    /// Too many datasets for the fixed-capacity buffer.
    CapacityExceeded,
}
```

Note: the existing std validation has `dataset_name: String` which requires the catalog indexer. The no_std version uses only `dataset_id: usize`. If the caller has access to a name mapping (Part 2), they can look up names after the fact. This keeps validation completely allocation-free.

### Complete example of Approach B

```rust
// src/core/validate.rs — available in both std and no_std

use crate::core::{PipelineInfo, StepInfo, DatasetRef};

/// Fixed-capacity set of dataset IDs (stack-allocated).
struct IdSet<const N: usize> {
    ids: [usize; N],
    len: usize,
}

impl<const N: usize> IdSet<N> {
    const fn new() -> Self {
        Self { ids: [0; N], len: 0 }
    }
    fn contains(&self, id: usize) -> bool {
        let mut i = 0;
        while i < self.len {
            if self.ids[i] == id { return true; }
            i += 1;
        }
        false
    }
    fn insert(&mut self, id: usize) -> bool {
        if self.contains(id) { return true; }
        if self.len >= N { return false; }  // capacity exceeded
        self.ids[self.len] = id;
        self.len += 1;
        true
    }
}

/// Validate a pipeline assuming sequential execution order.
///
/// N = max number of distinct datasets. Typical pipelines need N=32 or N=64.
/// Returns Ok(()) or the first error found.
pub fn validate_sequential<const N: usize>(
    pipe: &impl StepInfo,
) -> Result<(), NoStdValidationError> {
    let mut produced = IdSet::<N>::new();
    let mut consumed = IdSet::<N>::new();
    let mut result = Ok(());

    pipe.for_each_info(&mut |item| {
        if result.is_ok() {
            result = validate_item::<N>(item, &mut produced, &mut consumed);
        }
    });

    result
}

fn validate_item<const N: usize>(
    item: &dyn PipelineInfo,
    produced: &mut IdSet<N>,
    consumed: &mut IdSet<N>,
) -> Result<(), NoStdValidationError> {
    if item.is_leaf() {
        // Check all inputs are available
        let mut err = Ok(());
        item.for_each_input_id(&mut |d: &DatasetRef| {
            if err.is_err() { return; }
            if !consumed.insert(d.id) {
                err = Err(NoStdValidationError::CapacityExceeded);
                return;
            }
            if !d.is_param && !produced.contains(d.id) {
                err = Err(NoStdValidationError::InputNotProduced {
                    node_name: item.get_name(),
                    dataset_id: d.id,
                });
            }
        });
        err?;

        // Check all outputs are valid
        item.for_each_output_id(&mut |d: &DatasetRef| {
            if err.is_err() { return; }
            if d.is_param {
                err = Err(NoStdValidationError::ParamWritten {
                    node_name: item.get_name(),
                    dataset_id: d.id,
                });
                return;
            }
            if produced.contains(d.id) {
                err = Err(NoStdValidationError::DuplicateOutput {
                    node_name: item.get_name(),
                    dataset_id: d.id,
                });
                return;
            }
            if !produced.insert(d.id) {
                err = Err(NoStdValidationError::CapacityExceeded);
            }
        });
        err
    } else {
        // Pipeline node: validate children, then check contract
        let mut child_produced = IdSet::<N>::new();
        let mut child_consumed = IdSet::<N>::new();

        // Snapshot current produced set for children to see
        // (children can depend on things produced before this pipeline)
        let mut inner_produced = IdSet::<N>::new();
        // Copy parent's produced into inner
        let mut i = 0;
        while i < produced.len {
            inner_produced.insert(produced.ids[i]);
            i += 1;
        }

        let mut result = Ok(());
        item.for_each_child(&mut |child| {
            if result.is_ok() {
                result = validate_item::<N>(child, &mut inner_produced, &mut child_consumed);
            }
        });
        result?;

        // Merge newly produced datasets (those in inner_produced but not in produced)
        i = 0;
        while i < inner_produced.len {
            produced.insert(inner_produced.ids[i]);
            i += 1;
        }

        // Check pipeline contract: declared outputs are produced
        item.for_each_output_id(&mut |d: &DatasetRef| {
            if !inner_produced.contains(d.id) && !d.is_param {
                result = Err(NoStdValidationError::UnproducedPipelineOutput {
                    pipeline_name: item.get_name(),
                    dataset_id: d.id,
                });
            }
        });
        result?;

        // Check pipeline contract: declared inputs are consumed
        item.for_each_input_id(&mut |d: &DatasetRef| {
            if !child_consumed.contains(d.id) {
                result = Err(NoStdValidationError::UnusedPipelineInput {
                    pipeline_name: item.get_name(),
                    dataset_id: d.id,
                });
            }
        });
        result
    }
}
```

### Comparison: sequential ordering check vs. current std validation

| Property | Current std validation | Proposed no_std validation |
|----------|----------------------|---------------------------|
| Algorithm | Two-pass: collect all outputs, then check all inputs | Single-pass: walk in order, check as we go |
| Ordering check | None (doesn't verify sequential feasibility) | Core feature: inputs must be produced by earlier nodes |
| Collections | HashMap, HashSet, Vec (heap) | `[usize; N]` flat arrays (stack) |
| Pipeline contracts | Yes | Yes |
| Error detail | Includes `String` dataset names | Only `usize` dataset IDs + `&'static str` node names |
| Dependencies | std | none |

Note that the proposed no_std validation is actually **stronger** than the current std validation because it additionally verifies sequential ordering. The current std validation only checks that inputs are produced _somewhere_ — not that they're produced _before_ the consuming node. This means the no_std validator also catches ordering bugs that the std version currently misses.

---

## Part 2: Node/Dataset Names in no_std

### The problem

The `SequentialRunner` currently ignores `catalog` and `params` — it only uses `pipe`:

```rust
fn run<E>(&self, pipe: &impl Steps<E>, _catalog: &impl Serialize, _params: &impl Serialize) -> Result<(), E> {
```

This means hooks and error messages can reference `node.name` (`&'static str`) but not dataset names. The `catalog_indexer` provides dataset names but requires `HashMap<usize, String>` (std-only).

### What do we actually need names for?

1. **Hook callbacks** — `before_node_run`, `on_node_error`, etc. already receive `&dyn PipelineInfo` which has `get_name()` for the node name. But hooks might want to log _which dataset_ a node reads/writes.
2. **Validation error messages** — the no_std validator in Part 1 reports `dataset_id: usize`, which is an opaque pointer address. Having a name makes errors actionable ("node 'add_offset' missing input 'a'" vs "node 'add_offset' missing input 0x7ffc12345678").
3. **Debugging** — when a pipeline fails, knowing which dataset was involved helps.

### How the existing std catalog indexer works

The existing `catalog_indexer.rs` exploits a property of serde's `#[derive(Serialize)]`:

```rust
#[derive(Serialize)]
struct Catalog {
    a: CellDataset<i32>,
    b: CellDataset<i32>,
}
```

The derived `Serialize` impl generates code equivalent to:

```rust
fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct("Catalog", 2)?;
    state.serialize_field("a", &self.a)?;   // key is &'static str, value is &self.a
    state.serialize_field("b", &self.b)?;   // key is &'static str, value is &self.b
    state.end()
}
```

Two critical things happen in `serialize_field(key, value)`:
- `key` is `&'static str` — the field name, baked into the binary at compile time
- `value` is `&self.a` — a reference to the _same_ struct field that pipeline nodes hold references to

Because `ptr_to_id(&self.a)` gives the same pointer address whether called from serde or from a `Node`'s input/output tuple, we get a free `pointer_id → field_name` mapping. The std version stores this in `HashMap<usize, String>`.

### Approach: Flat array of `(usize, &'static str)` — the no_std serde indexer

The key insight: **we don't need `String` at all for flat catalogs**. The `key` parameter in `serialize_field` is already `&'static str`. We just store it directly.

#### Core data structure

```rust
/// Stack-allocated mapping from dataset pointer IDs to field names.
/// N = maximum number of datasets (catalog fields + param fields).
pub struct NoStdCatalogIndex<const N: usize> {
    entries: [(usize, &'static str); N],
    len: usize,
}

impl<const N: usize> NoStdCatalogIndex<N> {
    const fn new() -> Self {
        Self {
            entries: [(0, ""); N],
            len: 0,
        }
    }

    /// Look up the name for a dataset pointer ID.
    fn get(&self, ptr_id: usize) -> Option<&'static str> {
        let mut i = 0;
        while i < self.len {
            if self.entries[i].0 == ptr_id {
                return Some(self.entries[i].1);
            }
            i += 1;
        }
        None
    }

    /// Insert a mapping. Returns false if capacity exceeded.
    fn insert(&mut self, ptr_id: usize, name: &'static str) -> bool {
        // Overwrite if already present
        let mut i = 0;
        while i < self.len {
            if self.entries[i].0 == ptr_id {
                self.entries[i].1 = name;
                return true;
            }
            i += 1;
        }
        if self.len >= N { return false; }
        self.entries[self.len] = (ptr_id, name);
        self.len += 1;
        true
    }
}
```

This is essentially the same `IdSet` pattern from Part 1 but storing `(usize, &'static str)` pairs instead of bare `usize` values. Linear scan is fine — a pipeline with 50 datasets does 50 comparisons per lookup, which is negligible.

#### The no_std serde Serializer

The custom serializer is a near-copy of the existing `catalog_indexer.rs`, but stores into the flat array instead of a HashMap and skips prefix tracking (the main source of `String` allocation):

```rust
use serde::ser::{self, Serialize};
use crate::core::ptr_to_id;

/// Build a no_std catalog index from any struct that derives Serialize.
pub fn index_catalog_nostd<const N: usize>(catalog: &impl Serialize) -> NoStdCatalogIndex<N> {
    let mut indexer = NoStdIndexer {
        index: NoStdCatalogIndex::new(),
    };
    catalog.serialize(&mut indexer).ok();
    indexer.index
}

struct NoStdIndexer<const N: usize> {
    index: NoStdCatalogIndex<N>,
}
```

The critical `SerializeStruct` implementation:

```rust
impl<'a, const N: usize> ser::SerializeStruct for &'a mut NoStdIndexer<N> {
    type Ok = ();
    type Error = IndexerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,    // <-- this is the field name, already &'static str!
        value: &T,
    ) -> Result<(), Self::Error> {
        let ptr_id = ptr_to_id(value);
        self.index.insert(ptr_id, key);

        // Recurse into nested structs (their fields get their own keys)
        value.serialize(&mut **self).ok();
        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}
```

Compare this to the std version's `serialize_field`:

```rust
// std version (existing code in catalog_indexer.rs:134-147)
fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
    let ptr_id = ptr_to_id(value);
    let name = self.full_name(key);          // <-- allocates String: "prefix.key"
    self.names.insert(ptr_id, name.clone()); // <-- inserts into HashMap
    let prev_prefix = std::mem::replace(&mut self.prefix, name); // <-- swaps String
    value.serialize(&mut **self).ok();
    self.prefix = prev_prefix;               // <-- restores String
    Ok(())
}
```

The no_std version is _simpler_: no prefix tracking, no `String` allocation, no `mem::replace`. The trade-off is that nested struct fields lose their parent prefix.

The remaining serializer methods are all no-ops (identical to the existing std version — `serialize_bool`, `serialize_i32`, etc. all return `Ok(())`). The `Serializer` trait's `serialize_struct` method returns `Ok(self)`, and `serialize_newtype_struct` recurses via `value.serialize(self)`. Everything else is a no-op. This is the same pattern as the existing code — the no-op impls can even be shared or copy-pasted.

#### serde dependency: already no_std compatible

The `Cargo.toml` already has:

```toml
serde = { version = "1.0.228", default-features = false, features = ["derive"] }
```

`serde` with `default-features = false` works in no_std. The `Serialize` derive and `ser::Serializer` trait are all available. The `ser::Error` trait requires `core::fmt::Display`, which is available in no_std via `core::fmt`. No additional dependencies are needed.

#### Full example: indexing a flat catalog

```rust
use serde::Serialize;

#[derive(Serialize)]
struct Catalog {
    a: CellDataset<i32>,
    b: CellDataset<i32>,
    c: CellDataset<i32>,
}

#[derive(Serialize)]
struct Params {
    scale: Param<i32>,
    offset: Param<i32>,
}

let catalog = Catalog { a: CellDataset::new(), b: CellDataset::new(), c: CellDataset::new() };
let params = Params { scale: Param(3), offset: Param(10) };

// Index both catalog and params (N=8 is plenty for 5 fields)
let cat_index = index_catalog_nostd::<8>(&catalog);
let par_index = index_catalog_nostd::<8>(&params);

assert_eq!(cat_index.get(ptr_to_id(&catalog.a)), Some("a"));
assert_eq!(cat_index.get(ptr_to_id(&catalog.b)), Some("b"));
assert_eq!(cat_index.get(ptr_to_id(&catalog.c)), Some("c"));
assert_eq!(par_index.get(ptr_to_id(&params.scale)), Some("scale"));
assert_eq!(par_index.get(ptr_to_id(&params.offset)), Some("offset"));
```

#### Handling nested catalogs

For a nested catalog:

```rust
#[derive(Serialize)]
struct Inner {
    alpha: CellDataset<i32>,
    beta: CellDataset<i32>,
}

#[derive(Serialize)]
struct Catalog {
    inner: Inner,
    gamma: CellDataset<i32>,
}
```

The std indexer produces: `"inner.alpha"`, `"inner.beta"`, `"gamma"`.

The no_std flat-array indexer produces: `"alpha"`, `"beta"`, `"gamma"` — because there's no prefix tracking. Additionally, the entry for the `inner` field itself gets recorded (with the pointer to the `Inner` struct), but that's harmless — it just won't match any dataset ID since nodes reference `inner.alpha`, not `inner`.

**Is this a problem?** Only if two nested structs have fields with the same name:

```rust
#[derive(Serialize)]
struct Catalog {
    prices: Inner,    // has field "value"
    volumes: Inner,   // has field "value" — same &'static str!
}
```

In this case both `prices.value` and `volumes.value` would be recorded as just `"value"`. But they'd have _different_ pointer IDs, so both entries would exist in the flat array — `(ptr_of_prices_value, "value")` and `(ptr_of_volumes_value, "value")`. The lookup by pointer ID would still return the correct entry. The only downside is that the _name_ doesn't disambiguate which `"value"` it is. This is acceptable for error messages ("missing input 'value' in node 'foo'" is still useful) and can be documented.

**If disambiguation is needed later**: a prefix-tracking scheme can be added using a `&'static str` stack (array of `&'static str` segments) instead of `String` concatenation. But this adds complexity and isn't needed for the typical flat-catalog case.

#### Nested prefix tracking without allocation (optional extension)

If nested catalogs become common, here's how prefix tracking could work without `String`:

```rust
struct NoStdIndexer<const N: usize, const D: usize> {
    index: NoStdCatalogIndex<N>,
    prefix_stack: [&'static str; D],  // D = max nesting depth (e.g. 4)
    depth: usize,
}

impl<const N: usize, const D: usize> NoStdIndexer<N, D> {
    /// Build the dotted name by joining prefix segments.
    /// Returns only the leaf key if no nesting, or formats "a.b.c" style.
    ///
    /// Problem: we can't return a &'static str for "inner.alpha" because
    /// that string doesn't exist in the binary. We'd need to either:
    ///   (a) store the segments alongside each entry, or
    ///   (b) use heapless::String to build it, or
    ///   (c) accept leaf-only names
    fn current_prefix(&self) -> ??? { ... }
}
```

This shows why the simple approach (leaf-only names) is the right default: building dotted paths requires _some_ form of string concatenation, which in no_std means either `heapless::String` or a fixed-size buffer with `core::fmt::Write`. It's doable but adds complexity for a corner case. The leaf-only approach handles the common case cleanly.

#### Integration with validation errors

The `NoStdCatalogIndex` can enrich validation errors:

```rust
// After validation:
let index = index_catalog_nostd::<32>(&catalog);

match validate_sequential::<32>(&pipe) {
    Ok(()) => {},
    Err(e) => {
        // e.dataset_id is a usize — look up the name
        let name = index.get(e.dataset_id()).unwrap_or("<unknown>");
        // In no_std with a UART or debug probe:
        // "Validation error: node 'add_offset' missing input 'a'"
    }
}
```

#### Integration with hooks

A hook implementation that has access to a `NoStdCatalogIndex` can look up dataset names:

```rust
struct DebugHook<'a, const N: usize> {
    catalog_index: &'a NoStdCatalogIndex<N>,
}

impl<const N: usize> Hook for DebugHook<'_, N> {
    fn before_node_run(&self, n: &dyn PipelineInfo) {
        // Log node name (always available)
        // Node name: n.get_name() -> &'static str

        // Log input dataset names
        n.for_each_input_id(&mut |d: &DatasetRef| {
            let name = self.catalog_index.get(d.id).unwrap_or("?");
            // write to UART, defmt, etc.: "node 'add_offset' reads 'a'"
        });
    }
}
```

This is the same pattern the std `LoggingHook` would use, but with the no_std index type.

#### Integration with the sequential runner

The `SequentialRunner` currently receives `catalog` and `params` but ignores them in no_std. With the no_std indexer, it could optionally build the index and pass it to hooks:

```rust
// Option 1: user builds index externally (recommended — keeps Runner simple)
let index = index_catalog_nostd::<32>(&catalog);
let hook = DebugHook { catalog_index: &index };
let runner = SequentialRunner::new((hook,));
runner.run::<PondError>(&pipe, &catalog, &params)?;

// Option 2: runner builds index internally (adds const generic to Runner)
// Not recommended — pollutes the Runner type signature
```

Option 1 is recommended: the user builds the index and passes it into their hook. The runner doesn't need to know about the index at all. This keeps the `Runner` trait unchanged.

### Comparison of all approaches

| Approach | Dependencies | Nesting support | Complexity | Allocation |
|----------|-------------|----------------|------------|------------|
| A: heapless `FnvIndexMap` | heapless | Full (with `heapless::String`) | Medium | None (stack) |
| **B: Flat `(usize, &'static str)` array** | **none** | **Leaf names only** | **Low** | **None (stack)** |
| C: Manual registration | none | Full (user provides) | Low (user burden) | None (stack) |
| D: Names only in std | none | N/A | Zero | N/A |
| E: Node names only | none | N/A | Zero | N/A |

### Recommendation for Part 2

**Approach B (flat array)** is the right choice for no_std dataset names. It:

- Adds zero dependencies (serde is already present with `default-features = false`)
- Requires ~80 lines of code (the `NoStdCatalogIndex` struct + no_std `Serializer` impl)
- Reuses the exact same serde trick as the existing std indexer
- Works perfectly for flat catalogs (all current examples)
- Degrades gracefully for nested catalogs (leaf-only names, still usable)
- Can be upgraded to handle nesting later if needed

---

## Part 3: Concrete Complexity Assessment

### What would change

| Change | Files affected | New dependencies | Lines of code (est.) |
|--------|---------------|-----------------|---------------------|
| Add `src/core/validate.rs` with `IdSet` + `validate_sequential` | 1 new file + `src/core/mod.rs` | none | ~120 |
| Add `NoStdValidationError` enum | same file | none | ~30 |
| Wire into `SequentialRunner` (optional validation before run) | `src/runners/sequential.rs` | none | ~10 |
| Re-export from `src/lib.rs` | `src/lib.rs` | none | ~2 |
| **Total** | **3-4 files** | **none** | **~160** |

### What would NOT change

- The existing `graph` module stays as-is (std-only, used by ParallelRunner)
- The existing `catalog_indexer` stays as-is
- No changes to Node, Pipeline, or any traits
- No changes to datasets
- No new feature flags needed (validation is always available)

### The role of `heapless`

`heapless` is **not needed** for the recommended approach. The flat `[usize; N]` array is sufficient. If later we want:
- Hash-based lookup for large pipelines → use `heapless::FnvIndexSet`
- Dataset name mapping in no_std → use `heapless::FnvIndexMap` + `heapless::String`

These are independent, incremental upgrades that don't block the initial implementation.

---

## Part 4: Integration with the Sequential Runner

The validation could be integrated in two ways:

### Option 1: Explicit validation call (recommended)

Users call validation themselves before running:

```rust
let pipe = ( /* ... */ );

// Validate (compile-time const for max datasets)
validate_sequential::<64>(&pipe)?;

// Run
let runner = SequentialRunner::new(());
runner.run::<PondError>(&pipe, &catalog, &params)?;
```

**Pros:** User controls when/whether validation happens. No overhead in production if validation is skipped.

### Option 2: Validation inside the runner

The runner validates before executing:

```rust
impl Runner for SequentialRunner<H> {
    fn run<E>(&self, pipe: &impl Steps<E>, ...) -> Result<(), E> {
        validate_sequential::<64>(pipe).map_err(/* ... */)?;
        // ... execute ...
    }
}
```

**Cons:** Hard to parameterize `N` (the const generic). The runner would need a const generic parameter too, which ripples through the API. Also, validation errors would need to be convertible to `E`.

### Recommendation

**Option 1** — keep validation as a standalone function. It's simpler, more flexible, and doesn't complicate the runner's type signature.

---

## Summary

| Question | Answer |
|----------|--------|
| Is no_std graph validation feasible? | **Yes**, and it's simpler than the std version |
| Best approach? | Flat `[usize; N]` array, single-pass walk, ~160 lines |
| New dependencies needed? | **None** (heapless is optional, for future upgrades) |
| Does it check sequential ordering? | **Yes** — this is the core invariant it checks |
| Does it check pipeline contracts? | **Yes** — same checks as the std version |
| What about dataset names? | Use node names + dataset IDs in no_std; enrich with std `CatalogIndex` when available |
| Is heapless needed? | **Not for the initial implementation**; useful as a future upgrade path |
| Complexity impact? | Minimal — 1 new file, ~160 lines, no trait changes |
