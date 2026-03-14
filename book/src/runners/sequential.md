# Sequential Runner

The `SequentialRunner` executes pipeline steps one at a time in definition order.

## Overview

```rust,ignore
#[derive(Default)]
pub struct SequentialRunner;

impl Runner for SequentialRunner {
    fn name(&self) -> &'static str { "sequential" }
    // ...
}
```

- Available in both `std` and `no_std` environments
- Executes nodes in the exact order they appear in the steps tuple
- Recursively enters `Pipeline` containers, executing children in order
- Default runner when no `--runner` flag is specified

## Behavior

Given this pipeline:

```rust,ignore
(
    Node { name: "a", .. },
    Pipeline {
        name: "inner",
        steps: (
            Node { name: "b", .. },
            Node { name: "c", .. },
        ),
        ..
    },
    Node { name: "d", .. },
)
```

Execution order is: `a` → `b` → `c` → `d`.

Hook events fire in this order:

```text
before_node_run("a")
after_node_run("a")
before_pipeline_run("inner")
  before_node_run("b")
  after_node_run("b")
  before_node_run("c")
  after_node_run("c")
after_pipeline_run("inner")
before_node_run("d")
after_node_run("d")
```

## Error handling

If any node fails, execution stops immediately. No subsequent nodes are executed. The error propagates up through any enclosing `Pipeline` containers, firing `on_pipeline_error` at each level.

## no_std differences

In `no_std` builds:
- Dataset names are not resolved (no catalog indexer), so `ds.name` in hook callbacks is always `None`
- Error messages in `on_node_error` are the fixed string `"node error"` instead of the full error message

## When to use

Use the sequential runner when:
- Your nodes have strict ordering requirements
- You're in a `no_std` environment
- You want deterministic, predictable execution
- Debugging — sequential execution makes it easier to trace issues
