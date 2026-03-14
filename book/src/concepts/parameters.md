# Parameters

Parameters are read-only configuration values that feed into pipeline nodes. They are defined using `Param<T>`, a thin wrapper that implements the `Dataset` trait. For more on `Param`, nested parameters, and struct parameters, see the [Params & Catalog](../params_catalog/README.md) chapter.

## In the minimal example

The `Params` struct holds a single threshold value:

```rust,ignore
{{#include ../../../examples/minimal.rs:params}}
```

Configured via YAML:

```yaml
# params.yml
threshold: 0.5
```

The "report" node reads the threshold as one of its inputs. When `Param` appears in a node's `input` tuple, `.load()` clones the inner value. Because `Param::Error` is `Infallible`, this can never fail.

```rust,ignore
{{#include ../../../examples/minimal.rs:report_node}}
```

## Overriding from the CLI

When using the `App` framework, parameters can be overridden at runtime without editing YAML files:

```sh
$ my_app run --params threshold=0.8
```

Dot notation works for nested parameter structs:

```sh
$ my_app run --params model.learning_rate=0.01
```

See the [YAML Configuration](../app/yaml.md) chapter for details.
