# Contributing

Thank you for your interest in contributing to pondrs!

We aspire to keep the core library more or less stable and do not accept major changes to its architecture or API surface. That said, we welcome contributions in the following areas:

1. **New dataset implementations** that are relevant for the larger community and not tied to a single commercial offering.
2. **Bugfixes**, soundness fixes, performance improvements, and compile time improvements.

If you're unsure whether your contribution fits, please open an issue to discuss it before starting work.

## Before opening a PR

CI runs these; running them locally first is faster than a round trip:

```bash
cargo build                              # default (all features)
cargo build --no-default-features --lib  # the no_std path still compiles
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --no-default-features --lib -- -D warnings
```

The lint set lives in `[lints]` in `Cargo.toml`: `clippy::pedantic` with a short
allow-list, plus `use_self` and a few `restriction` lints — notably
`undocumented_unsafe_blocks`, so every `unsafe` block needs a `// SAFETY:`
comment naming the invariant it relies on. Each opt-out in that section carries
a comment explaining itself; if a lint is wrong for one specific site, prefer a
local `#[allow(..., reason = "...")]` over widening the crate-level list.
