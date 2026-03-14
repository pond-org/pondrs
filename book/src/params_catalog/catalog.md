# Catalog

The catalog is a plain Rust struct that groups all datasets used by a pipeline. Any struct that derives `Serialize` and `Deserialize` and contains dataset fields works as a catalog — there is no special trait to implement.

## Dataset fields

Each field in the catalog is a dataset. The field names become the dataset names used in logging, visualization, and error messages. The framework discovers them automatically via serde serialization.

```rust,ignore
#[derive(Serialize, Deserialize)]
struct Catalog {
    readings: PolarsCsvDataset,
    summary: MemoryDataset<f64>,
    report: JsonDataset,
}
```

## Nested catalogs

Catalogs can nest other structs for organization. This is useful when a pipeline has many datasets that fall into logical groups:

```rust,ignore
#[derive(Serialize, Deserialize)]
struct InputData {
    raw_readings: PolarsCsvDataset,
    reference: YamlDataset,
}

#[derive(Serialize, Deserialize)]
struct Catalog {
    input: InputData,
    output: JsonDataset,
    intermediate: MemoryDataset<f64>,
}
```

The discovered dataset names use dot-separated paths: `input.raw_readings`, `input.reference`, `output`, and `intermediate`. These names appear in logs, hooks, and the viz dashboard.

The corresponding YAML mirrors the nesting:

```yaml
input:
  raw_readings:
    path: data/raw.csv
  reference:
    path: data/ref.yml
output:
  path: data/output.json
intermediate: {}
```

## Naming conventions

The framework uses serde struct names to distinguish leaf datasets from container structs:

- Types whose serde name ends with `"Dataset"` are treated as **leaf datasets** — the indexer stops recursing
- `Param` is treated as a leaf by name
- All other struct names are treated as **containers** — the indexer recurses into their fields

This means custom dataset types should follow the `*Dataset` naming convention (e.g. `TextDataset`, `MyCustomDataset`). Container structs like nested catalogs or parameter groups must **not** end with "Dataset".

## Catalog overrides

Dataset configuration can be overridden from the CLI using dot notation:

```sh
$ my_app run --catalog output.path=/tmp/result.json
$ my_app run --catalog input.raw_readings.separator=";"
```

See [YAML Configuration](../app/yaml.md) for the full details on overrides.
