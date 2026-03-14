# Catalog

The catalog is a plain Rust struct that groups all datasets used by a pipeline. It is not a special type — any struct that derives `Serialize` and `Deserialize` and contains dataset fields works as a catalog.

## In the minimal example

```rust,ignore
{{#include ../../../examples/minimal.rs:catalog}}
```

Each field is a dataset. The field names become the dataset names used in logging, visualization, and error messages — the framework discovers them automatically via serde serialization.

## YAML configuration

The catalog struct is deserialized from a YAML file. Each field maps to a YAML key, and the dataset type determines what configuration is needed:

```yaml
# catalog.yml
readings:
  path: data/readings.csv
  separator: ","
summary: {}
report:
  path: data/report.json
```

- **File-backed datasets** (like `PolarsCsvDataset`, `JsonDataset`) need at least a `path`.
- **In-memory datasets** (like `MemoryDataset`) use an empty mapping `{}` — they have no persistent configuration.
- **Parameters** live in a separate params struct and file, not in the catalog.

## Loading the catalog

When using `App::from_yaml` or `App::from_args`, the catalog is loaded and deserialized automatically:

```rust,ignore
{{#include ../../../examples/minimal.rs:app}}
```

You can also load it manually:

```rust,ignore
let contents = std::fs::read_to_string("catalog.yml")?;
let catalog: Catalog = serde_yaml::from_str(&contents)?;
```

For nested catalogs, naming conventions, and catalog overrides, see the [Params & Catalog](../params_catalog/README.md) chapter.
