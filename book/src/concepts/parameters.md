# Parameters

```rust,no_run
#[derive(Serialize, Deserialize)]
struct Params {
    warn_threshold: Param<u16>,
    crit_threshold: Param<u16>,
}
```

```yaml
warn_threshold: 500
crit_threshold: 900
```

