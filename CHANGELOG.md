# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.3.0] - 2026-05-05

### Added
- `LazyDataset<D>` wrapper — defers load and save to call time via `Lazy<T, E> = Box<dyn FnOnce() -> Result<T, E> + Send>` thunks
- `LazyPartitionedDataset<D>` type alias (`PartitionedDataset<LazyDataset<D>>`) for lazy partitioned workflows
- `PartitionedNode` — applies a per-element function across all partitions of a `PartitionedDataset`, with automatic thunk bridging for any eager/lazy combination
- `Thunk<T>`, `IntoThunk<T>`, `FromThunk<T>` traits for converting between eager values and lazy thunks
- `FileDataset::prefer_parallel()` method — controls whether `PartitionedDataset` uses rayon for parallel save (`LazyDataset` returns `true`)
- `FileDataset::list_entries()` method — lists partition entry names (default scans directory; overridable for non-filesystem storage)
- Book chapter on lazy datasets, `LazyPartitionedDataset`, and `PartitionedNode`

### Changed
- **Breaking:** renamed `PipelineInfo` trait to `StepInfo` (per-node metadata) and `StepInfo` trait to `PipelineInfo` (collection-level metadata) — the old names were swapped relative to their meaning
- **Breaking:** `PartitionedDataset` and `LazyPartitionedDataset` moved from `polars` feature to `std` feature — they work with any `FileDataset`, not just Polars types
- **Breaking:** `LazyPartitionedDataset` is now a type alias for `PartitionedDataset<LazyDataset<D>>` instead of a standalone struct; `Lazy<T>` replaced by `Lazy<T, E>` (a `FnOnce` closure instead of a `Fn`-based wrapper struct)
- **Breaking:** `ParallelRunner` now uses a rayon thread pool instead of `std::thread::scope`; configurable via `ParallelRunner::new(num_threads)` (`ParallelRunner::default()` uses all CPUs)
- `PartitionedDataset` load/save logic consolidated — uses `FileDataset::list_entries()` and `FileDataset::ensure_parent_dir()` instead of inline filesystem code
- Reduced debug info (`split-debuginfo = "unpacked"`, `debuginfo = "line-tables-only"`) to speed up dev builds

## [0.2.5] - 2026-03-30

### Added
- Node filtering for partial pipeline runs: `--nodes`, `--from-nodes`, `--to-nodes` CLI flags
- `NodeFilter` enum and `filter_steps()` function for programmatic filtering
- `PondError::NodeNotFound` variant for invalid node names
- Blanket `PipelineInfo` and `RunnableStep<E>` impls for references

## [0.2.4] - 2026-03-25

### Added
- `CONTRIBUTING.md` with contribution guidelines
- AI disclosure in README
- Link to examples repository in README

## [0.2.3] - 2026-03-24

### Added
- `FileDataset::ensure_parent_dir()` default method — automatically creates parent directories before saving
- All built-in file datasets (`TextDataset`, `JsonDataset`, `YamlDataset`, `PolarsCsvDataset`, `PolarsParquetDataset`, `PlotlyDataset`, `ImageDataset`) now call `ensure_parent_dir()` in `save()`

## [0.2.2] - 2026-03-24

### Fixed
- `MemoryDataset<T>` no longer requires `T: Default`

### Changed
- Trimmed dependency features to reduce compile times:
  - `polars`: disabled defaults, enabled only `csv`, `parquet`, `fmt`, `dtype-slim`
  - `image`: disabled defaults, enabled only `png`, `jpeg`, `tiff`, `bmp`
  - `ureq`: disabled defaults (removed TLS, not needed for localhost)
- Limited `mold` linker thread count to avoid memory exhaustion

## [0.2.1] - 2026-03-23

### Fixed
- `PolarsExcelDataset` now reads integer columns as `Int64` instead of `Float64` (Excel stores all numbers as floats)
- Pipeline validation now correctly verifies that all consumed datasets inside pipelines are declared as inputs

## [0.2.0] - 2026-03-21

### Added
- `TemplatedCatalog` for defining multiple datasets with the same structure, with `Split` and `Join` pipeline nodes
- `PolarsExcelDataset` for reading/writing Excel files
- `StepVec` for type-erased dynamic pipeline construction
- `Debug` impls for public types

## [0.1.0] - 2025-03-10

Initial release.
