# Datasets

This chapter covers all built-in dataset types and how to implement your own.

The `Dataset` trait is introduced in [A minimal pipeline — Datasets](../concepts/datasets.md). Here we go deeper into each concrete implementation.

- **[Custom Datasets](./custom_datasets.md)** — implementing the `Dataset` trait for your own types
- **[Param](./param.md)** — read-only parameter values
- **[Memory Dataset](./memory.md)** — thread-safe in-memory storage
- **[Cell Dataset](./cell.md)** — stack-friendly `no_std` storage
- **[Partitioned Dataset](./partitioned.md)** — directory of files loaded as a map
- **[Cache Dataset](./cache.md)** — caching wrapper for any dataset
- **[List of Datasets](./other.md)** — quick reference for all built-in types
