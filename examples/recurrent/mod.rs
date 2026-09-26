//! Shared pipeline definition for the recurrent example and integration tests.
//!
//! Demonstrates: `RecurrentNode` unrolling a loop into one node per iteration,
//! a `Vec<_>` catalog field written as `{count, template}` via `expand_indexed`,
//! and a downstream node consuming the last iteration's output.

use serde::{Deserialize, Serialize};
use serde_json::json;

use pondrs::datasets::{JsonDataset, Param, TextDataset};
use pondrs::{Node, RecurrentNode, Steps};

// ANCHOR: types
#[derive(Serialize, Deserialize)]
pub struct Catalog {
    /// The starting point, read by iteration 0.
    pub init_weights: TextDataset,
    /// One checkpoint per iteration: `checkpoints.0`, `checkpoints.1`, …
    #[serde(deserialize_with = "pondrs::datasets::expand_indexed")]
    pub checkpoints: Vec<TextDataset>,
    pub report: JsonDataset,
}

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub target: Param<f64>,
    pub learning_rate: Param<f64>,
}
// ANCHOR_END: types

// ANCHOR: nodes
/// One gradient step on `(w - target)^2`.
pub fn step(weights: String, target: f64, lr: f64) -> (String,) {
    let w: f64 = weights.trim().parse().unwrap();
    let grad = 2.0 * (w - target);
    ((w - lr * grad).to_string(),)
}
// ANCHOR_END: nodes

// ANCHOR: pipeline
pub fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<pondrs::error::PondError> + 'a {
    (
        RecurrentNode {
            name: "train",
            state: &cat.checkpoints,
            init: &cat.init_weights,
            input: |prev, _cur| (prev, &params.target, &params.learning_rate),
            output: |cur| (cur,),
            func: step,
        }
        .build(),
        Node {
            name: "report",
            input: (cat.checkpoints.last().expect("at least one iteration"),),
            output: (&cat.report,),
            func: |w: String| (json!({ "final_weights": w.trim().parse::<f64>().unwrap() }),),
        },
    )
}
// ANCHOR_END: pipeline

pub fn write_fixtures(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    let d = dir.display();
    std::fs::write(dir.join("init.txt"), "0").unwrap();
    std::fs::write(
        dir.join("catalog.yml"),
        format!(
            "init_weights:\n  path: \"{d}/init.txt\"\n\
             checkpoints:\n  count: 5\n  template:\n    path: \"{d}/ckpt/epoch_{{i}}.txt\"\n\
             report:\n  path: \"{d}/report.json\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("params.yml"), "target: 10.0\nlearning_rate: 0.25\n").unwrap();
}

pub fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("recurrent_data")
}
