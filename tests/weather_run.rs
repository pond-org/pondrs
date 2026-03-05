#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]
#![allow(dead_code)]

//! Integration test for the weather pipeline, exercising the full PondApp
//! entrypoint via `try_main_from`.
//!
//! The weather pipeline's `validate_reports` node intentionally fails, so the
//! test asserts that the pipeline returns an error.

#[path = "../examples/weather/mod.rs"]
mod weather;

use pondrs::app::PondApp;

use weather::{WeatherApp, write_fixtures};

#[test]
fn weather_pipeline_returns_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let cat_path = dir.path().join("catalog.yml");
    let params_path = dir.path().join("params.yml");

    let result = WeatherApp::try_main_from([
        "test",
        "--catalog-path", cat_path.to_str().unwrap(),
        "--params-path",  params_path.to_str().unwrap(),
        "run",
        "--runner", "parallel",
    ]);

    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("east station"), "expected validation error about east station, got: {msg}");
}

#[test]
fn weather_pipeline_check_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let cat_path = dir.path().join("catalog.yml");
    let params_path = dir.path().join("params.yml");

    WeatherApp::try_main_from([
        "test",
        "--catalog-path", cat_path.to_str().unwrap(),
        "--params-path",  params_path.to_str().unwrap(),
        "check",
    ]).unwrap();
}
