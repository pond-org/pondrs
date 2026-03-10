# Catalog

```rust,no_run
#[derive(Serialize, Deserialize)]
struct Catalog {
    csv_text: TextDataset,
    csv_data: PolarsCsvDataset,
    chart: PlotlyDataset,
}
```

```yaml
csv_text:
  path: data/fruits.csv
csv_data:
  path: data/fruits.csv
  separator: ','
  has_header: true
chart:
  path: data/fruit_chart.json
```

```rust,no_run
let contents = fs::read_to_string("catalog.yml")?;
let catalog: Catalog = serde_yaml::from_str(&contents)?;
let df = csv_data.load()?;
```
