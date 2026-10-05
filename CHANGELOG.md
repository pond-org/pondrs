# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.4.0] - 2026-10-05

### Added
- **A dataset's `Error` now converts directly into the pipeline error type `E`.** Write one `thiserror` enum with a `#[from]` variant for `PondError` and one per custom dataset error, and a `Dataset::Error` reaches it with its type and `source()` chain intact:

  ```rust
  #[derive(Debug, thiserror::Error)]
  enum AppError {
      #[error(transparent)] Pond(#[from] PondError),
      #[error(transparent)] Gps(#[from] GpsError),
  }
  ```

  Previously the chain was `Dataset::Error → PondError → E`, so a custom dataset error had to be stringified through `PondError::Custom` — and in `no_std`, where `Custom` does not exist, there was no valid conversion target at all. No `PondError: From<YourError>` impl is needed any more, and none should be written.
- `NodeInputMeta` / `NodeOutputMeta` traits — the non-generic metadata halves of `NodeInput<E>` / `NodeOutput<E>`, mirroring the `StepMeta`/`Step<E>` split.
- `PondError::Other(Box<dyn core::error::Error + Send + Sync>)` and the `PondError::other(e)` constructor — wraps a foreign error losslessly, preserving `Display`, the `source()` chain, and `downcast_ref`. Prefer it over `PondError::Custom`, which flattens an error to its message.
- `PondError::Message(&'static str)` — the `no_std`-usable counterpart of `Custom`/`Other`, which both need an allocator.
- **Catalog/dataset identity diagnostics.** `pondrs` identifies datasets by the address of their catalog field, which had three silent failure modes: a dataset type not following the `*Dataset` naming convention gets an interior field name in logs and viz; a dataset the serde walk never reaches gets no name at all; and two zero-sized datasets collapse into one DAG node. All three are now surfaced by `check`, which still exits 0 for the two that are only naming problems:
  - `CheckError::AliasedDatasets` — two datasets of different types at one address. Runs as a new *first* pass in `check()`, before the ordering checks: an alias otherwise shows up as a spurious `DuplicateOutput` / `InputNotProduced`.
  - `CheckWarning` (`ZeroSizedDataset`, `UnconventionalDatasetType`) plus `StepsMeta::for_each_warning` / `for_each_warning_with_capacity::<N>`. Both are provided methods built on `for_each_meta`, and report through a callback rather than a collection — so they work in `no_std` with no allocator. Existing `StepsMeta` implementors need no changes.
  - `CatalogWarning` (`UnnamedDataset`, `MisresolvedName`, `UnusedCatalogEntry`) plus `check_catalog(pipe, catalog, params, report)` — the cross-check between the pipeline's datasets and the names the catalog walk resolves for them. `std` only; pass the unfiltered pipeline. `MisresolvedName` names both the wrong resolved name and the correct one. `UnusedCatalogEntry` reports catalog and param entries no node touches, sorted by name so the output is deterministic; it is the one warning that can be wrong rather than merely incomplete, and its message names the two legitimate causes (a param read while *building* the pipeline, and a catalog shared between pipelines). `check` skips `check_catalog` when a node filter is active, since a filtered-out node's datasets would all look unused.
  - `my_app check` prints all of these and then a warning count. Detection only: no resolved name changes.
- `DatasetMeta::is_zero_sized()` — whether the concrete dataset type is zero-sized. A ZST dataset has no reliable address-based identity.
- **`RecurrentNode`** (`std`) — a loop unrolled into one real `Node` per iteration. `state: &[S]` holds one element per iteration; iteration `k` reads `state[k-1]` (or `init`) and writes `state[k]` through `input`/`output` projection closures, so an element may be a bare dataset or a sub-catalog carrying per-iteration extras. `.build()` returns an `Unrolled` group whose children are named `"{name}/{k}"`, so `--from-nodes train/47`, `CacheHook` and node hooks all address iterations individually. The group's input/output contract is derived from its children rather than declared. See the new book chapter and `examples/recurrent_app.rs`.
- **The catalog indexer names `Vec` elements by index** (`epochs.0.weights`, `epochs.1.weights`, …), the way it already named map entries by key. A `Vec<S>` catalog field is how a `RecurrentNode` gets one distinct dataset per iteration. Tuples are still not indexed.
- `pondrs::datasets::expand_indexed` — a `deserialize_with` helper that builds a `Vec<S>` from `{count, template}` with `{i}` substituted per element (optional `placeholder:` key), and also accepts a plain sequence so catalogs round-trip through `App::with_cli`. `--catalog field.count=N` then changes the iteration count.
- **`RetentionHook`** (`std`) — keeps only the most recent saves of datasets whose names match a pattern (`RetentionHook::new("catalog.epochs.*.weights", 3)`), deleting older ones as new ones are saved; `.keep_every(n)` never removes saves whose index is a multiple of `n`. For write-through checkpoints in a `RecurrentNode` epoch chain, where N full checkpoints would otherwise fill a disk.
- `Dataset::remover()` / `DatasetMeta::remover()` (`std`) — an owned handle (`Remover`) that deletes the dataset's stored value; owned so a hook can take it at save time and call it later. Default `None`. File datasets delete their file (via the new `FileDataset::file_remover()` helper), `PartitionedDataset` removes its entries through the inner dataset's remover, and `LazyDataset` / `CacheDataset` delegate, `CacheDataset` also clearing its memory. Read-only datasets (`Param`, `PolarsExcelDataset`) have none.
- `CacheDataset::take` (`#[serde(default)]`, or `.with_take(true)`) — the first load after a save moves the value out of memory instead of cloning it; later loads read the inner dataset. For single-consumer values such as epoch checkpoints, so memory holds only what has not been consumed yet.
- `Node`, `Pipeline`, `Alias` and `PartitionedNode` take any `AsRef<str> + Send + Sync` as `name`, via a new trailing type parameter defaulting to `&'static str`. Steps generated in a loop can now have distinct names (`format!("train/{k}")`); previously every node built by a `DynSteps` loop shared one name, which `--from-nodes` could not address and which made `CacheHook` share one cache key across all of them.

### Changed
- `FileDataset::file_content_hash` (and so `content_hash` of every file dataset and `CacheHook`'s keys) now includes the file size alongside path and mtime, catching a rewrite whose mtime was preserved (`rsync -t`, `cp -p`) as long as the size changed. Existing `.pondcache` keys no longer match, so every cached node re-runs once.
- `CacheDataset`'s `Dataset` impl requires `D::LoadItem: Send`. Any `CacheDataset` usable in a pipeline already satisfied it (`DatasetMeta` requires `Sync`).
- **Breaking:** `StepMeta::name()` returns `&str` instead of `&'static str`. Custom `StepMeta` impls change their signature; hooks that stored `name()` beyond the call must copy it (`to_string()`).
- **Breaking:** `CheckError`, `CheckWarning` and `CatalogWarning` gained a lifetime (`CheckError<'a>`, …): their `node_name` / `pipeline_name` fields borrow from the checked steps, and `check()` returns `Result<(), CheckError<'_>>`. Bind the pipeline to a local before calling `check()` on it if the error must outlive the statement.
- `check()` and `for_each_warning()` are unbounded under `std` (heap-backed bookkeeping) — previously `check()` returned `CapacityExceeded` past 20 datasets, and `App`'s `check` subcommand had no way to raise it. The fixed-capacity path is unchanged under `no_std`, and `check_with_capacity::<N>()` / `for_each_warning_with_capacity::<N>()` still use it explicitly.
- **Breaking:** `PondError` is now `#[non_exhaustive]`. Which variants exist already depended on the resolved feature set, so downstream code must not match it exhaustively; add a `_` arm.
- **Breaking:** `NodeInput` / `NodeOutput` gained an `E` parameter and shed their metadata methods to `NodeInputMeta` / `NodeOutputMeta`. `Node` and `Pipeline` bound their `Input`/`Output` on the `Meta` traits, so closure-signature diagnostics still fire at the `Node { .. }` literal; only the error-conversion check moves to where `E` is named, matching how node function errors already behaved.
- **Breaking:** `DatasetInput` / `DatasetOutput` gained an `Error` associated type, and `load_input` / `save_output` became generic over `E`. Custom port adapters need updating; plain dataset references and `EachField` are unaffected at use sites.
- **Breaking:** `Param<T>::Error` is now `PondError` instead of `Infallible`. `Param` appears in nearly every pipeline, and under the new bound `Infallible` would make every user error type owe a `From<Infallible>` impl — coherence rules out blanketing around it. `load()` still never fails. Prefer `PondError` over `Infallible` in your own datasets for the same reason.
- **Breaking:** `PolarsExcelDataset::SaveItem` is now `Never` instead of `DataFrame`. Excel support has always been read-only — `save()` was an `unimplemented!()` that panicked at run time. Naming the dataset as a node output is now a compile error instead, matching `Param`.
- `HookControl::merge` and `App::with_command` are `#[must_use]`; both return a value that is the whole point of the call.
- The crate now lints under `clippy::pedantic` (minus a documented allow-list) plus `use_self` and a few `restriction` lints, enforced in CI with `-D warnings`. The lint set lives in `[lints]` in `Cargo.toml`; see CONTRIBUTING.md. Nothing in the public API changed as a result beyond the two entries above.
- **Breaking:** `Thunk<T>` removed — it was an alias for exactly the same type as `Lazy<T, E>`, which is the name users already write in node signatures. Use `Lazy<T, E>`. The bridging traits are renamed to match: `IntoThunk<T>` → `IntoLazy<T, E>` (`into_thunk` → `into_lazy`), `FromThunk<T>` → `FromLazy<T, E>` (`from_thunk` → `from_lazy`); both now live in `datasets::lazy` alongside `LazyDataset`. The added `E` parameter also unpins `PartitionedNode` from `PondError`, so a partitioned node's function may now return a custom error type just as a plain `Node`'s may.
- **Breaking:** partitioned loads are repeatable and ordered, for per-epoch data loading with a reproducible order. `LazyDataset::LoadItem` is now `Loader<T, E> = Arc<dyn Fn() -> Result<T, E> + Send + Sync>`, which re-reads its file on every call (nothing is cached) and can be cloned across threads; `LazyDataset` now requires `D: Sync`. The save side and `PartitionedNode`'s chaining stay the one-shot `Lazy<T, E>`. `PartitionedDataset`'s load and save items are now `BTreeMap<String, _>`, iterated in entry-name order, instead of `HashMap`. Migrate by changing `HashMap` → `BTreeMap` in node signatures over partitioned datasets, and `Lazy` → `Loader` for lazy inputs; function bodies (`loader()?`, `for (k, v) in map`) are unchanged. `PartitionedNode` functions need no changes.

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
