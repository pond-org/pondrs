# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

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
