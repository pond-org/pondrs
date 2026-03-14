# Parameters

Parameters are read-only configuration values that feed into pipeline nodes. They are defined using `Param<T>`, a thin wrapper that implements the `Dataset` trait.

## How Param works

```rust,ignore
#[derive(Debug, Serialize, Deserialize)]
pub struct Param<T: Clone>(pub T);
```

`Param` implements `Dataset` with:

- `LoadItem = T` — returns a clone of the inner value
- `SaveItem = ()` — writing is forbidden; the pipeline validator rejects any node that writes to a `Param`
- `Error = Infallible` — loading always succeeds

Because `is_param()` returns `true`, the viz dashboard and pipeline check treat parameters differently from regular datasets.

## Nested parameters

Parameters can be organized into nested structs for clarity. Each struct level must derive `Serialize` and `Deserialize`:

```rust,ignore
#[derive(Serialize, Deserialize)]
struct ModelParams {
    learning_rate: Param<f64>,
    epochs: Param<u32>,
}

#[derive(Serialize, Deserialize)]
struct Params {
    model: ModelParams,
    verbose: Param<bool>,
}
```

```yaml
# params.yml
model:
  learning_rate: 0.01
  epochs: 100
verbose: true
```

Nested parameters can be overridden from the CLI with dot notation:

```sh
$ my_app run --params model.learning_rate=0.001
```

## Struct parameters

`Param<T>` works with any `T: Clone + Serialize + Deserialize`, including custom structs:

```rust,ignore
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BaselinePeriod {
    start_month: u32,
    end_month: u32,
}

#[derive(Serialize, Deserialize)]
struct Params {
    baseline: Param<BaselinePeriod>,
}
```

```yaml
baseline:
  start_month: 1
  end_month: 12
```

When a node loads this parameter, it receives the full `BaselinePeriod` struct as its argument.

## no_std

`Param` is available in `no_std` environments — it requires no feature flags and uses no allocation. Only `T: Clone + Serialize` is needed.
