//! Tests for the PipelineFn trait and its blanket impl for Fn.

use pondrs::app::App;
use pondrs::datasets::{MemoryDataset, Param};
use pondrs::error::PondError;
use pondrs::pipeline::{Node, Steps};
use pondrs::runners::SequentialRunner;
use serde::Serialize;

// ---------- Test types ----------

#[derive(Serialize)]
struct MyCatalog {
    input: MemoryDataset<f64>,
    output: MemoryDataset<f64>,
}

#[derive(Serialize)]
struct MyParams {
    scale: Param<f64>,
}

// ---------- Pipeline functions ----------

fn my_pipeline<'a>(
    cat: &'a MyCatalog,
    params: &'a MyParams,
) -> impl Steps<PondError> + 'a {
    (Node {
        name: "multiply",
        func: |x: f64, scale: f64| -> (f64,) { (x * scale,) },
        input: (&cat.input, &params.scale),
        output: (&cat.output,),
    },)
}

fn multi_node_pipeline<'a>(
    cat: &'a MyCatalog,
    params: &'a MyParams,
) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "step1",
            func: |x: f64, scale: f64| -> (f64,) { (x * scale,) },
            input: (&cat.input, &params.scale),
            output: (&cat.output,),
        },
        Node {
            name: "step2",
            func: |x: f64| -> (f64,) { (x + 1.0,) },
            input: (&cat.output,),
            output: (&cat.output,),
        },
    )
}

// ---------- Tests ----------

#[test]
fn test_app_run_with_named_fn() {
    let app = App::new(
        MyCatalog {
            input: MemoryDataset::default(),
            output: MemoryDataset::default(),
        },
        MyParams {
            scale: Param(2.0),
        },
    )
    .with_runners((SequentialRunner,));

    let result: Result<(), PondError> = app.execute(my_pipeline);
    // DatasetNotLoaded is expected since MemoryDataset::default() has no data
    match &result {
        Ok(()) => {}
        Err(PondError::DatasetNotLoaded) => {} // expected
        Err(e) => core::panic!("unexpected error: {e}"),
    }
}

#[test]
fn test_app_run_multi_node() {
    let app = App::new(
        MyCatalog {
            input: MemoryDataset::default(),
            output: MemoryDataset::default(),
        },
        MyParams {
            scale: Param(5.0),
        },
    )
    .with_runners((SequentialRunner,));

    let result: Result<(), PondError> = app.execute(multi_node_pipeline);
    match &result {
        Ok(()) => {}
        Err(PondError::DatasetNotLoaded) => {} // expected
        Err(e) => core::panic!("unexpected error: {e}"),
    }
}
