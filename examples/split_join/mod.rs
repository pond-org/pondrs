//! Shared pipeline definition for the split/join example.
//!
//! Demonstrates: TemplatedCatalog, Split, Join, StepVec,
//! fan-out/fan-in patterns with per-item parallel processing.

use std::collections::HashMap;

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use pondrs::datasets::{MemoryDataset, Param, PolarsCsvDataset, JsonDataset};
use pondrs::{Join, Node, RunnableStep, Split, StepVec, TemplatedCatalog};

// ANCHOR: types
// ---------------------------------------------------------------------------
// Per-store catalog entry (expanded from YAML template)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreCatalog {
    pub inventory: PolarsCsvDataset,
    pub total_value: MemoryDataset<f64>,
}

// ---------------------------------------------------------------------------
// Catalog and params
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct Catalog {
    pub all_inventory: PolarsCsvDataset,
    pub grouped: MemoryDataset<HashMap<String, DataFrame>>,
    pub stores: TemplatedCatalog<StoreCatalog>,
    pub store_values: MemoryDataset<HashMap<String, f64>>,
    pub report: JsonDataset,
}

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub low_stock_threshold: Param<i64>,
}

// ANCHOR_END: types

// ANCHOR: nodes
// ---------------------------------------------------------------------------
// Node functions
// ---------------------------------------------------------------------------

/// Read the combined CSV and group rows by the "store" column.
fn group_by_store(df: DataFrame) -> Result<(HashMap<String, DataFrame>,), PolarsError> {
    let store_col = df.column("store")?.str()?;
    let unique: Vec<String> = store_col
        .into_no_null_iter()
        .map(|s| s.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut map = HashMap::new();
    for store in &unique {
        let mask = store_col.equal(store.as_str());
        map.insert(store.clone(), df.filter(&mask)?);
    }
    Ok((map,))
}

/// Compute the total stock value for a single store.
///
/// For each row: value = quantity * unit_price.
/// Items below `low_stock_threshold` are flagged but still counted.
fn compute_store_value(df: DataFrame, threshold: i64) -> (f64,) {
    let qty = df.column("quantity").unwrap().i64().unwrap();
    let price = df.column("unit_price").unwrap().f64().unwrap();

    let mut total = 0.0;
    let mut low_stock_items = 0;
    for i in 0..df.height() {
        let q = qty.get(i).unwrap_or(0);
        let p = price.get(i).unwrap_or(0.0);
        total += q as f64 * p;
        if q < threshold {
            low_stock_items += 1;
        }
    }

    if low_stock_items > 0 {
        log::warn!(
            "{low_stock_items} item(s) below stock threshold of {threshold}"
        );
    }
    (total,)
}

/// Build a JSON comparison report from per-store totals.
fn build_report(store_values: HashMap<String, f64>) -> (Value,) {
    let grand_total: f64 = store_values.values().sum();
    let mut stores: Vec<Value> = store_values
        .iter()
        .map(|(name, &value)| {
            json!({
                "store": name,
                "total_value": (value * 100.0).round() / 100.0,
                "share_pct": ((value / grand_total * 10000.0).round() / 100.0),
            })
        })
        .collect();
    stores.sort_by(|a, b| {
        b["total_value"]
            .as_f64()
            .unwrap()
            .partial_cmp(&a["total_value"].as_f64().unwrap())
            .unwrap()
    });

    let report = json!({
        "grand_total": (grand_total * 100.0).round() / 100.0,
        "stores": stores,
    });
    (report,)
}

// ANCHOR_END: nodes

// ANCHOR: pipeline
// ---------------------------------------------------------------------------
// Pipeline function
// ---------------------------------------------------------------------------

pub fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> StepVec<'a> {
    // Step 1: group the combined CSV into a HashMap by store.
    let mut steps: StepVec<'a> = vec![
        Node {
            name: "group_by_store",
            func: group_by_store,
            input: (&cat.all_inventory,),
            output: (&cat.grouped,),
        }
        .boxed(),
    ];

    // Step 2: Split distributes per-store DataFrames to individual CSV files.
    steps.push(
        Split {
            name: "split_stores",
            input: &cat.grouped,
            catalog: &cat.stores,
            field: |s: &StoreCatalog| &s.inventory,
        }
        .boxed(),
    );

    // Step 3: per-store processing — dynamically build a node for each store.
    for (_, store) in cat.stores.iter() {
        steps.push(
            Node {
                name: "compute_store_value",
                func: compute_store_value,
                input: (&store.inventory, &params.low_stock_threshold),
                output: (&store.total_value,),
            }
            .boxed(),
        );
    }

    // Step 4: Join collects per-store totals back into a HashMap.
    steps.push(
        Join {
            name: "join_values",
            catalog: &cat.stores,
            field: |s: &StoreCatalog| &s.total_value,
            output: &cat.store_values,
        }
        .boxed(),
    );

    // Step 5: build a comparison report from the joined values.
    steps.push(
        Node {
            name: "build_report",
            func: build_report,
            input: (&cat.store_values,),
            output: (&cat.report,),
        }
        .boxed(),
    );

    steps
}

// ANCHOR_END: pipeline

// ---------------------------------------------------------------------------
// Fixture generation
// ---------------------------------------------------------------------------

pub fn data_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("examples").join("split_join_data")
}

pub fn write_fixtures(dir: &std::path::Path) {
    use std::fs;

    fs::create_dir_all(dir).unwrap();

    // Combined inventory CSV with three stores.
    fs::write(
        dir.join("all_inventory.csv"),
        "\
store,product,quantity,unit_price
north,Widget A,120,9.99
north,Widget B,45,24.50
north,Widget C,8,149.00
north,Widget D,200,3.50
south,Widget A,90,9.99
south,Widget B,60,24.50
south,Widget C,15,149.00
south,Widget D,5,3.50
east,Widget A,75,9.99
east,Widget B,30,24.50
east,Widget C,22,149.00
east,Widget D,180,3.50
",
    )
    .unwrap();

    fs::write(
        dir.join("catalog.yml"),
        format!(
            "\
all_inventory:
  path: {d}/all_inventory.csv
grouped: {{}}
stores:
  placeholder: \"store\"
  template:
    inventory:
      path: \"{d}/{{store}}_inventory.csv\"
    total_value: {{}}
  names: [north, south, east]
store_values: {{}}
report:
  path: {d}/report.json
",
            d = dir.display()
        ),
    )
    .unwrap();

    fs::write(dir.join("params.yml"), "low_stock_threshold: 10\n").unwrap();
}
