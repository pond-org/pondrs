#![allow(dead_code)]

//! Integration test for the sales pipeline via App::from_args + dispatch.

#[path = "../examples/sales/mod.rs"]
mod sales;

use pondrs::hooks::LoggingHook;

use sales::{sales_pipeline, write_fixtures};

#[test]
fn sales_pipeline_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let cat_path = dir.path().join("catalog.yml");
    let params_path = dir.path().join("params.yml");

    let app = pondrs::app::App::from_args([
        "test",
        "--catalog-path", cat_path.to_str().unwrap(),
        "--params-path",  params_path.to_str().unwrap(),
        "run",
    ]).unwrap()
    .with_hooks((LoggingHook::new(),));

    app.dispatch(sales_pipeline).unwrap();
}

#[test]
fn sales_pipeline_check_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let cat_path = dir.path().join("catalog.yml");
    let params_path = dir.path().join("params.yml");

    let app = pondrs::app::App::from_args([
        "test",
        "--catalog-path", cat_path.to_str().unwrap(),
        "--params-path",  params_path.to_str().unwrap(),
        "check",
    ]).unwrap();

    app.dispatch(sales_pipeline).unwrap();
}
