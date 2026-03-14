# Param

`Param<T>` is a read-only dataset wrapper for configuration values. It is the primary way to pass parameters into pipeline nodes.

## Definition

```rust,ignore
#[derive(Debug, Serialize, Deserialize)]
pub struct Param<T: Clone>(pub T);

impl<T: Clone + Serialize> Dataset for Param<T> {
    type LoadItem = T;
    type SaveItem = ();
    type Error = Infallible;

    fn load(&self) -> Result<T, Infallible> { Ok(self.0.clone()) }
    fn save(&self, _: ()) -> Result<(), Infallible> { unreachable!() }
    fn is_param(&self) -> bool { true }
}
```

Key properties:

- **Loading always succeeds** — `Error = Infallible`
- **Writing is forbidden** — `save()` is unreachable; the pipeline validator (`check()`) rejects any node that writes to a `Param`
- **`is_param()` returns `true`** — used by the validator and visualization to distinguish params from data

## Usage

```rust,ignore
#[derive(Serialize, Deserialize)]
struct Params {
    threshold: Param<f64>,
    max_retries: Param<u32>,
}

Node {
    name: "filter",
    func: |value: f64, threshold: f64| {
        (value >= threshold,)
    },
    input: (&cat.value, &params.threshold),
    output: (&cat.passed,),
}
```

## YAML

```yaml
threshold: 0.5
max_retries: 3
```

`Param<T>` deserializes directly from the YAML value — no wrapping object needed.

## Visualization

In the viz dashboard, parameters appear as distinct node shapes, separate from datasets. They are also shown in the left panel's "Parameters" section.

## no_std

`Param` is available in `no_std` — it requires no feature flags and uses no allocation.
