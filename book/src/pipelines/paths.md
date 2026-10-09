# Step Paths

Every step has a **path**: the names of its enclosing pipelines followed by its
own name, joined with `/`. In

```rust,ignore
(
    Node { name: "split", .. },
    Pipeline {
        name: "north",
        steps: (Node { name: "compute", .. },),
        ..
    },
    Pipeline {
        name: "south",
        steps: (Node { name: "compute", .. },),
        ..
    },
)
```

the two `compute` nodes share a name but not a path: they are
`north/compute` and `south/compute`. Top-level steps' paths are just their
names (`split`). A flat tuple of steps adds no segment; only `Pipeline`s (and
other groups, such as the one a [`RecurrentNode`](./recurrent.md) unrolls into)
do.

## Paths are unique

[`check`](./check.md) enforces two rules on names:

- **Siblings have distinct names.** Two steps in the same pipeline, or two
  top-level steps, may not share a name (`CheckError::DuplicateStepName`).
- **No `/` in a name** (`CheckError::InvalidStepName`), since `/` separates
  path segments.

Together these make every path in a pipeline unique, so a path identifies one
step. When you build steps in a loop, give each a distinct name or, as the
[fan-out example](../examples/split_join.md) does, put each in a pipeline named
after its item.

## Where paths are used

- **Hooks** receive each node and pipeline under its full path: inside a hook,
  `n.name()` is `north/compute`, not `compute`. This is what keeps per-node
  hook state apart — `CacheHook`'s cache entries, `LoggingHook`'s lines,
  `VizHook`'s live status. Calling `name()` on a step yourself still gives its
  local name.
- **Node filters** (`run --nodes`, `--from-nodes`, `--to-nodes`) match a path
  or any suffix of it that starts at a `/`:
  `--nodes north/compute` selects one node, `--nodes compute` selects both.
  `orth/compute` matches nothing.
- **Viz** shows full paths on the canvas and in the detail panel, and the
  last segment in the node tree, whose indentation shows the rest.

> **`no_std`:** without an allocator there is nowhere to build a path, so on
> `no_std` hooks receive the local name. The `check` rules still apply.
