# Pipelines & Nodes

This chapter covers nodes and pipelines in more depth — the `NodeInput`/`NodeOutput` traits, the `Pipeline` struct for grouping nodes, and pipeline validation.

- **[Nodes](./nodes.md)** — `NodeInput`, `NodeOutput`, `CompatibleOutput`, and side-effect nodes
- **[Pipeline](./pipeline.md)** — the `Pipeline` struct for grouping nodes with input/output contracts
- **[Dynamic Pipelines](./dynamic.md)** — runtime step composition with `DynSteps`
- **[Fan-out & Fan-in](./split_join.md)** — fan-out/fan-in patterns with `EachField` and `TemplatedCatalog`
- **[Recurrent Nodes](./recurrent.md)** — loops unrolled into one node per iteration with `RecurrentNode`
- **[Check](./check.md)** — runtime validation of pipeline structure
