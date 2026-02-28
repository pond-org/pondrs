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

1. **Hook callbacks** — `before_node_run`, `on_node_error`, etc. already receive `&dyn PipelineInfo` which has `get_name()` for the node name. Dataset names would enrich logging.
2. **Validation error messages** — the no_std validator in Part 1 reports `dataset_id: usize`, which is an opaque pointer address. Having a name would make errors actionable.
3. **Debugging** — when a pipeline fails, knowing which dataset was involved helps.

### Approach A: no_std catalog indexer with `heapless`

Port the existing serde-based approach to use `heapless::FnvIndexMap<usize, heapless::String<M>, N>`.

```rust
use heapless::{FnvIndexMap, String as HString};

pub struct NoStdCatalogIndex<const N: usize, const M: usize> {
    names: FnvIndexMap<usize, HString<M>, N>,
}
```

Where `N` = max entries, `M` = max name length (e.g. 64 bytes).

**Pros:**
- Same serde-based approach, proven pattern
- Hash lookup for ID → name

**Cons:**
- Two const generics (`N` for capacity, `M` for string length)
- Requires `heapless` dependency
- The serde `Serializer` implementation needs porting (the prefix tracking uses `String` concatenation, needs `heapless::String` formatting)
- Still somewhat heavy for what it does

### Approach B: Flat array of `(usize, &'static str)` via serde

Same serde trick, but store `&'static str` field names directly. The key insight: serde's `serialize_field` receives `key: &'static str` — the field name is already a static string from the derived `Serialize` impl. We just can't build dotted paths ("inner.alpha") without allocation.

```rust
struct NoStdCatalogIndex<const N: usize> {
    entries: [(usize, &'static str); N],
    len: usize,
}
```

For a flat catalog like `struct Catalog { a: ..., b: ... }`, serde gives us `key = "a"` and `key = "b"` — perfect. For nested catalogs, we'd get `key = "alpha"` for `inner.alpha`, losing the prefix. This is acceptable if catalogs are flat (which they are in all current examples), and can be documented as a limitation.

**Pros:**
- Zero allocations, zero dependencies
- `&'static str` field names come free from serde
- Very simple implementation

**Cons:**
- Loses dotted prefix for nested catalogs (reports "alpha" instead of "inner.alpha")
- Const generic for capacity
- Still requires serde (already a dependency, and works with `default-features = false`)

### Approach C: Don't use serde at all — manual name registration

Add a trait or method for users to register dataset names explicitly:

```rust
// Users implement this:
impl Catalog {
    fn register_names<const N: usize>(&self) -> NameMap<N> {
        let mut map = NameMap::new();
        map.insert(ptr_to_id(&self.a), "a");
        map.insert(ptr_to_id(&self.b), "b");
        map
    }
}
```

**Pros:**
- No serde dependency for naming
- Full control over names (can include prefixes)
- Simple to understand

**Cons:**
- Manual and error-prone — names can go out of sync with struct fields
- Boilerplate for users
- Loses the elegance of the current automatic serde-based approach

### Approach D: Provide names only in std, accept IDs-only in no_std

The simplest approach: don't try to port the catalog indexer. In no_std, validation errors and hooks report `dataset_id: usize` (the pointer address). In std, the caller can optionally enrich errors with names after the fact.

```rust
// no_std: validate returns raw IDs
let err = validate_sequential::<64>(&pipe);

// std: enrich with names if desired
#[cfg(feature = "std")]
if let Err(e) = &err {
    let index = index_catalog(&catalog);
    eprintln!("Validation error: {}", e.display_with(&index));
}
```

**Pros:**
- Zero added complexity in no_std
- No new dependencies
- Clean separation of concerns

**Cons:**
- In no_std, error messages show raw pointer addresses (e.g. `dataset_id: 0x7ffc12345678`)
- Less useful for debugging in pure no_std environments

### Approach E: Use node names only (already available)

The sequential runner already has `node.get_name() -> &'static str`. For validation errors, the node name + dataset position ("node 'add_offset' input #1") may be sufficient without needing dataset names at all.

```rust
NoStdValidationError::InputNotProduced {
    node_name: "add_offset",
    input_index: 1,  // "the second input of this node"
    dataset_id: 0x...,
}
```

Combined with the fact that the user wrote the pipeline definition and can see which input #1 of "add_offset" refers to, this may be adequate.

**Pros:**
- Zero added complexity
- No new types, traits, or dependencies
- Node names are always available (`&'static str`)

**Cons:**
- Requires the user to mentally map "input #1" → dataset name
- Less friendly for large pipelines

### Recommendation for Part 2

**Start with Approach D + E**: don't port the catalog indexer. Use node names (already available) plus dataset IDs in no_std errors. If std is enabled, offer an optional `.display_with(index)` to enrich errors with dataset names.

If experience shows that no_std users genuinely need dataset names (embedded systems with logging, etc.), then **Approach B** (flat array of `(usize, &'static str)`) is the right next step — it's zero-dependency and works for flat catalogs.

Approach A (heapless maps) is overkill for this problem given the alternatives.

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
