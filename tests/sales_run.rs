#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]
#![allow(dead_code)]

//! Integration test for the sales pipeline, exercising the full PondApp
//! entrypoint via `try_main_from`.

#[path = "../examples/sales/mod.rs"]
mod sales;

use pondrs::app::PondApp;

use sales::{SalesApp, write_fixtures};

#[test]
fn sales_pipeline_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let cat_path = dir.path().join("catalog.yml");
    let params_path = dir.path().join("params.yml");

    SalesApp::try_main_from([
        "test",
        "--catalog-path", cat_path.to_str().unwrap(),
        "--params-path",  params_path.to_str().unwrap(),
        "run",
    ]).unwrap();
}

#[test]
fn sales_pipeline_check_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let cat_path = dir.path().join("catalog.yml");
    let params_path = dir.path().join("params.yml");

    SalesApp::try_main_from([
        "test",
        "--catalog-path", cat_path.to_str().unwrap(),
        "--params-path",  params_path.to_str().unwrap(),
        "check",
    ]).unwrap();
}
