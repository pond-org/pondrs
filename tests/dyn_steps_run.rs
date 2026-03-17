#![allow(dead_code)]

//! Integration tests for the dyn_steps pipeline.
//!
//! Covers both the normal run (report included) and the conditional case
//! where `include_report: false` causes the report node to be skipped.

#[path = "../examples/dyn_steps/mod.rs"]
mod dyn_steps;

use dyn_steps::{pipeline, write_fixtures};

#[test]
fn dyn_steps_pipeline_check_succeeds() {
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
fn dyn_steps_pipeline_run_with_report() {
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

    let report = std::fs::read_to_string(dir.path().join("report.json")).unwrap();
    assert!(report.contains("\"mean\""));
    assert!(report.contains("\"passed\""));
}

#[test]
fn dyn_steps_pipeline_run_without_report() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    pondrs::app::App::from_args([
        "test",
        "--catalog-path", dir.path().join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.path().join("params.yml").to_str().unwrap(),
        "run",
        "--params", "include_report=false",
    ])
    .unwrap()
    .dispatch(pipeline)
    .unwrap();

    // report node was skipped — file should not exist
    assert!(!dir.path().join("report.json").exists());
}
