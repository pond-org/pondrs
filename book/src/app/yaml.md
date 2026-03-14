# YAML Configuration

pondrs uses YAML files for catalog and parameter configuration. The `App` framework loads, patches, and deserializes these files automatically.

## Catalog YAML

The catalog file maps dataset field names to their configuration:

```yaml
# catalog.yml
readings:
  path: data/readings.csv
  separator: ","
summary: {}
report:
  path: data/report.json
```

Each top-level key corresponds to a field in your catalog struct. The configuration under each key is deserialized into the dataset type for that field.

- File-backed datasets need at least a `path`
- In-memory datasets use `{}`
- Nested catalog structs create nested YAML sections

### Nested catalogs

```rust,ignore
#[derive(Serialize, Deserialize)]
struct InputData {
    raw: PolarsCsvDataset,
    reference: YamlDataset,
}

#[derive(Serialize, Deserialize)]
struct Catalog {
    input: InputData,
    output: JsonDataset,
}
```

```yaml
input:
  raw:
    path: data/raw.csv
    separator: ","
  reference:
    path: data/ref.yml
output:
  path: data/output.json
```

## Parameters YAML

```yaml
# params.yml
threshold: 0.5
model:
  learning_rate: 0.01
  epochs: 100
```

Each key maps to a `Param<T>` field. Nested structs create nested YAML sections.

## CLI overrides

Both catalog and parameter values can be overridden from the command line using dot notation:

```sh
# Override parameters
$ my_app run --params threshold=0.8
$ my_app run --params model.learning_rate=0.001

# Override catalog configuration
$ my_app run --catalog output.path=/tmp/result.json
$ my_app run --catalog readings.separator=;

# Multiple overrides
$ my_app run --params threshold=0.8 --params model.epochs=200
```

### How overrides work

1. The YAML file is loaded into a `serde_yaml::Value` tree
2. Each `KEY=VALUE` override is parsed and applied to the tree using dot notation
3. Values are parsed as YAML scalars (auto-detecting numbers, bools, strings, null)
4. The patched tree is deserialized into the target struct

Overrides create missing intermediate keys if needed — you can override deeply nested values even if the parent keys don't exist in the file.

## File paths

Default paths (when using `App::from_args` or `App::from_cli`):
- Catalog: `conf/base/catalog.yml`
- Params: `conf/base/parameters.yml`

Override with CLI flags:

```sh
$ my_app --catalog-path my/catalog.yml --params-path my/params.yml run
```

Or specify paths directly with `App::from_yaml`:

```rust,ignore
App::from_yaml("conf/catalog.yml", "conf/params.yml")?
```
