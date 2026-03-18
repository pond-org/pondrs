//! Shared pipeline definition for the dyn_steps_app example and integration tests.

#[path = "../minimal_fixtures/mod.rs"]
mod minimal_fixtures;

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use pondrs::datasets::{JsonDataset, MemoryDataset, Param, PolarsCsvDataset};
use pondrs::{Node, RunnableStep, StepVec};

// ANCHOR: types
#[derive(Serialize, Deserialize)]
pub struct Catalog {
    pub readings: PolarsCsvDataset,
    pub summary: MemoryDataset<f64>,
    pub report: JsonDataset,
}

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub threshold: Param<f64>,
    pub include_report: Param<bool>,
}
// ANCHOR_END: types

// ANCHOR: pipeline
pub fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> StepVec<'a> {
    let mut steps = vec![
        Node {
            name: "summarize",
            func: |df: DataFrame| {
                let mean = df.column("value").unwrap().f64().unwrap().mean().unwrap();
                (mean,)
            },
            input: (&cat.readings,),
            output: (&cat.summary,),
        }
        .boxed(),
    ];

    if params.include_report.0 {
        steps.push(
            Node {
                name: "report",
                func: |mean: f64, threshold: f64| {
                    (json!({ "mean": mean, "passed": mean >= threshold }),)
                },
                input: (&cat.summary, &params.threshold),
                output: (&cat.report,),
            }
            .boxed(),
        );
    }

    steps
}
// ANCHOR_END: pipeline

pub fn write_fixtures(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    minimal_fixtures::write_readings_csv(dir);
    minimal_fixtures::write_catalog_yml(dir);
    std::fs::write(dir.join("params.yml"), "threshold: 0.5\ninclude_report: true\n").unwrap();
}
