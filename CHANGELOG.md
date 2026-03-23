# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

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
