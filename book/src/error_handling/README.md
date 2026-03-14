# Error handling

pondrs uses Rust's standard `Result` type for error propagation. The framework provides `PondError` for infrastructure errors, while letting you define your own error type for domain-specific failures.

- **[Error Type](./error_type.md)** — `PondError`, custom error types, and the `From<PondError>` requirement
- **[Node Errors](./nodes.md)** — how fallible node functions propagate errors
- **[Dataset Errors](./datasets.md)** — the `Dataset::Error` associated type and adding custom error variants
