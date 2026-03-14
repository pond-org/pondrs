# A minimal pipeline

This chapter walks through the core concepts of pondrs using a minimal example. The same example appears on the [introduction page](../introduction.md) — here we break it apart and explain each piece in detail.

The example reads a CSV of sensor readings, computes the mean, compares it against a threshold parameter, and writes a JSON report. The following sections explain each concept:

- **[Parameters](./parameters.md)** — read-only values loaded from YAML (`Param<f64>`)
- **[Datasets](./datasets.md)** — the `Dataset` trait and the concrete types used here (`PolarsCsvDataset`, `MemoryDataset`, `JsonDataset`)
- **[Catalog](./catalog.md)** — the struct that groups datasets together
- **[Nodes](./nodes.md)** — the `Node` struct that connects a function to its input/output datasets
- **[Steps](./steps.md)** — how nodes are composed into a sequence that the runner can execute
- **[App](./app.md)** — how `App` ties everything together and provides CLI dispatch
