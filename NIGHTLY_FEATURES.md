# Nightly Rust Features Used in pondrs

This document enumerates all nightly features required by the `pondrs` crate, describes how
they are used, and assesses their stabilization outlook.

## Summary

| Feature | Tracking Issue | Status | Estimated Stabilization |
|---------|---------------|--------|------------------------|
| `unboxed_closures` | [#29625](https://github.com/rust-lang/rust/issues/29625) | Blocked on design concerns | 2028+ (unlikely before variadic generics) |
| `fn_traits` | [#29625](https://github.com/rust-lang/rust/issues/29625) | Blocked on design concerns | 2028+ (same blocker) |
| `tuple_trait` | None (`issue = "none"`) | Internal compiler feature | Unclear; tied to `fn_traits` |
| `impl_trait_in_assoc_type` | [#63063](https://github.com/rust-lang/rust/issues/63063) | Blocked on `-Znext-solver` | Late 2026 – 2027 |

---

## 1. `unboxed_closures` + `fn_traits`

**Tracking issue:** [rust-lang/rust#29625](https://github.com/rust-lang/rust/issues/29625)
(open since November 2015)

### How pondrs uses them

These two features enable user-defined types to implement the `Fn`, `FnMut`, and `FnOnce`
traits, and to call them via the `rust-call` ABI. In pondrs, `Node` uses `F: Fn<Input::Args>`
bounds and calls closures with `Fn::call(&self.func, args)`, enabling compile-time variadic
node functions:

```rust
// src/pipeline/node.rs
pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: Fn<Input::Args>,
{ ... }

// Direct Fn::call with tuple-unpacked arguments
let result = Fn::call(&self.func, args);
```

This allows nodes to accept closures with any arity (`|v| ...`, `|v, a| ...`, etc.) while
remaining zero-cost and `no_std`-compatible.

### Stabilization prognosis

**Unlikely before 2028.** The primary blocker is a design concern: the Rust team is uncertain
whether the current tuple-based `Args` parameter is the right long-term design, given that
variadic generics may arrive around 2030. The issue carries the `S-tracking-design-concerns`
label. There is no active stabilization push.

---

## 2. `tuple_trait`

**Tracking issue:** None — marked `#[unstable(feature = "tuple_trait", issue = "none")]` in
`core::marker`.

### How pondrs uses it

The `Tuple` marker trait is used as a bound on `NodeInput`, `NodeOutput`, `StepInfo`, `Steps`,
and `IntoNodeResult` to ensure that only tuple types flow through the pipeline:

```rust
// src/pipeline/traits.rs
pub trait NodeInput: Tuple {
    type Args: Tuple;
    ...
}

// src/pipeline/into_result.rs
pub trait IntoNodeResult<O: Tuple, E> { ... }
```

This prevents non-tuple types from being accidentally used as node inputs/outputs.

### Stabilization prognosis

**Unclear; effectively tied to `fn_traits`.** The `Tuple` trait is an internal compiler marker
with no public tracking issue. It exists primarily to support the `Fn` trait family. It is
unlikely to stabilize independently or before `fn_traits` does.

---

## 3. `impl_trait_in_assoc_type`

**Tracking issue:** [rust-lang/rust#63063](https://github.com/rust-lang/rust/issues/63063)
(partial stabilization of RFC 2515)

### How pondrs uses it

Used for opaque return types in functions that construct pipeline steps, and for closure return
types in runner callbacks:

```rust
// src/main.rs
fn construct_pipe1(params: &Parameters, catalog: &Catalog) -> impl Steps<PondError> { ... }

// src/runners/sequential.rs
fn make_dataset_callback<'a, E>(...) -> impl FnMut(&DatasetRef, DatasetEvent) + 'a { ... }
```

This avoids boxing or naming complex nested tuple/closure types.

### Stabilization prognosis

**Late 2026 – 2027.** The primary blocker is the next-generation trait solver
([`-Znext-solver`](https://github.com/rust-lang/rust/issues/107374)). Per the Rust project
goals, the global solver is targeted for stabilization in 2026, with preparation work
completed during 2025H2. Once the solver stabilizes, ATPIT can proceed through FCP. This is
the most likely feature to stabilize soon.

---

## Risk Assessment

### High risk (core architecture dependency)
- **`unboxed_closures` / `fn_traits` / `tuple_trait`** — These three features form the
  backbone of the `Node` execution model. They have no stabilization timeline and depend on
  unresolved language design questions (variadic generics). If the Rust team decides on a
  different design, pondrs may need significant refactoring.

### Moderate risk (convenience dependency)
- **`impl_trait_in_assoc_type`** — Used for ergonomic return types. Could be worked around
  with named types or `Box<dyn>` at the cost of some verbosity (and heap allocation for the
  latter). Most likely to stabilize in the near-term.

## Potential Mitigation Strategies

1. **For `fn_traits`/`unboxed_closures`:** Consider a macro-based approach that generates
   concrete impls for each supported arity (0–10 args) without requiring `Fn<Args>` bounds.
   This is how many stable Rust libraries (e.g., `axum`, `bevy`) handle variadic function
   signatures.

2. **For `tuple_trait`:** Replace `Tuple` bounds with a custom sealed marker trait
   implemented via macro for supported tuple sizes.

3. **For `impl_trait_in_assoc_type`:** Use concrete named types or `Box<dyn Trait>` as
   fallback. This feature has the best stabilization outlook, so waiting may also be viable.
