//! Minimal example used in the book introduction.
//!
//! Usage:
//!   cargo run --example minimal

#[path = "minimal_fixtures/mod.rs"]
mod minimal_fixtures;

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use pondrs::datasets::{JsonDataset, MemoryDataset, Param, PolarsCsvDataset};
use pondrs::error::PondError;
use pondrs::{Node, Steps};

// ANCHOR: types
// ANCHOR: catalog
#[derive(Serialize, Deserialize)]
struct Catalog {
    readings: PolarsCsvDataset,
    summary: MemoryDataset<f64>,
    report: JsonDataset,
}
// ANCHOR_END: catalog

// ANCHOR: params
#[derive(Serialize, Deserialize)]
struct Params {
    threshold: Param<f64>,
}
// ANCHOR_END: params
// ANCHOR_END: types

// ANCHOR: pipeline
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (
        // ANCHOR: summarize_node
        Node {
            name: "summarize",
            func: |df: DataFrame| {
                let mean = df.column("value").unwrap().f64().unwrap().mean().unwrap();
                (mean,)
            },
            input: (&cat.readings,),
            output: (&cat.summary,),
        },
        // ANCHOR_END: summarize_node
        // ANCHOR: report_node
        Node {
            name: "report",
            func: |mean: f64, threshold: f64| {
                (json!({ "mean": mean, "passed": mean >= threshold }),)
            },
            input: (&cat.summary, &params.threshold),
            output: (&cat.report,),
        },
        // ANCHOR_END: report_node
    )
}
// ANCHOR_END: pipeline

fn main() -> Result<(), PondError> {
    let dir = data_dir();
    write_fixtures(&dir);

    // ANCHOR: app
    pondrs::app::App::from_yaml(
        dir.join("catalog.yml").to_str().unwrap(),
        dir.join("params.yml").to_str().unwrap(),
    )?
    .with_args(std::env::args_os())?
    .dispatch(pipeline)
    // ANCHOR_END: app
}

fn data_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("examples").join("minimal_data")
}

fn write_fixtures(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    minimal_fixtures::write_readings_csv(dir);
    minimal_fixtures::write_catalog_yml(dir);
    std::fs::write(dir.join("params.yml"), "threshold: 0.5\n").unwrap();
}
