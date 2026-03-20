#![allow(dead_code)]

//! Integration tests for the split/join pipeline.

#[path = "../examples/split_join/mod.rs"]
mod split_join;

use split_join::{pipeline, write_fixtures};

#[test]
fn split_join_pipeline_check_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    pondrs::app::App::from_args([
        "test",
        "--catalog-path", dir.path().join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.path().join("params.yml").to_str().unwrap(),
        "check",
    ])
    .unwrap()
    .dispatch(pipeline)
    .unwrap();
}

#[test]
fn split_join_pipeline_run_produces_report() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    pondrs::app::App::from_args([
        "test",
        "--catalog-path", dir.path().join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.path().join("params.yml").to_str().unwrap(),
        "run",
    ])
    .unwrap()
    .dispatch(pipeline)
    .unwrap();

    // Per-store CSV files should exist.
    assert!(dir.path().join("north_inventory.csv").exists());
    assert!(dir.path().join("south_inventory.csv").exists());
    assert!(dir.path().join("east_inventory.csv").exists());

    // Final report should contain all three stores.
    let report = std::fs::read_to_string(dir.path().join("report.json")).unwrap();
    assert!(report.contains("\"grand_total\""));
    assert!(report.contains("north"));
    assert!(report.contains("south"));
    assert!(report.contains("east"));
}

#[test]
fn split_join_pipeline_run_with_param_override() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    pondrs::app::App::from_args([
        "test",
        "--catalog-path", dir.path().join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.path().join("params.yml").to_str().unwrap(),
        "run",
        "--params", "low_stock_threshold=50",
    ])
    .unwrap()
    .dispatch(pipeline)
    .unwrap();

    // Should still produce a valid report.
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("report.json")).unwrap())
            .unwrap();
    assert!(report["grand_total"].as_f64().unwrap() > 0.0);
    assert_eq!(report["stores"].as_array().unwrap().len(), 3);
}
