# 🤔 pondrs

[![CI](https://github.com/pond-org/pondrs/actions/workflows/ci.yml/badge.svg)](https://github.com/pond-org/pondrs/actions/workflows/ci.yml)

[Repo](https://github.com/pond-org/pondrs) | [Crate](https://crates.io/crates/pondrs) | [Docs](https://docs.rs/pondrs/latest/pondrs) | [Book](https://pond-org.github.io/pondrs) | [Examples](https://github.com/pond-org/pondrs-examples)

**Pipelines over Nodes & Datasets** — a Rust pipeline execution library, heavily inspired by [Kedro](https://github.com/kedro-org/kedro).

<!-- ANCHOR: example -->
## Example

Define your catalog and params as structs, with datasets backed by files or memory:

```rust,ignore
#[derive(Serialize, Deserialize)]
struct Catalog {
    readings: PolarsCsvDataset,
    summary: MemoryDataset<f64>,
    report: JsonDataset,
}

#[derive(Serialize, Deserialize)]
struct Params {
    threshold: Param<f64>,
}
```

Write a pipeline function that wires nodes together through shared datasets:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "summarize",
            func: |df: DataFrame| {
                let mean = df.column("value").unwrap().f64().unwrap().mean().unwrap();
                (mean,)
            },
            input: (&cat.readings,),
            output: (&cat.summary,),
        },
        Node {
            name: "report",
            func: |mean: f64, threshold: f64| {
                (json!({ "mean": mean, "passed": mean >= threshold }),)
            },
            input: (&cat.summary, &params.threshold),
            output: (&cat.report,),
        },
    )
}
```

Configure your catalog and params via YAML and run with the built-in CLI:

```yaml
# conf/base/catalog.yml
readings:
  path: data/readings.csv
  separator: ","
summary: {}
report:
  path: data/report.json
```

```yaml
# conf/base/parameters.yml
threshold: 0.5
```

```rust,ignore
fn main() -> Result<(), PondError> {
    App::from_args(std::env::args_os())?
        .dispatch(pipeline)
}
```

```sh
$ my_app run
$ my_app run --params threshold=0.8   # override params from CLI
$ my_app check                        # validate pipeline DAG
$ my_app viz                          # interactive pipeline visualization
```
<!-- ANCHOR_END: example -->

## AI Disclosure

This library was designed and architected by humans. Implementation was carried out by an AI coding agent under close human supervision and review.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
