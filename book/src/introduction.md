# pondrs

**Pipelines of Nodes & Datasets** — a Rust pipeline execution library, heavily inspired by [Kedro](https://github.com/kedro-org/kedro).

## Example

Define your catalog and params as structs, with datasets backed by files or memory:

```rust,ignore
{{#include ../../examples/minimal.rs:types}}
```

Write a pipeline function that wires nodes together through shared datasets:

```rust,ignore
{{#include ../../examples/minimal.rs:pipeline}}
```

Configure your catalog and params via YAML and run with the built-in CLI:

```yaml
# catalog.yml
readings:
  path: data/readings.csv
  separator: ","
summary: {}
report:
  path: data/report.json
```

```yaml
# params.yml
threshold: 0.5
```

```rust,ignore
{{#include ../../examples/minimal.rs:app}}
```

```sh
$ my_app run
$ my_app run --params threshold=0.8   # override params from CLI
$ my_app check                        # validate pipeline DAG
$ my_app viz                          # interactive pipeline visualization
```

## Pipeline visualization

<a href="assets/minimal_viz.html" target="_blank">Open fullscreen</a>

<iframe src="assets/minimal_viz.html" style="width:100%; height:600px; border:1px solid #ccc; border-radius:4px;"></iframe>
