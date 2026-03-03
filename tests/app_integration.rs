#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Integration tests for the PondApp trait and app infrastructure.

use std::fs;

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use pondrs::app::config::{apply_overrides, deserialize_config, load_yaml};
use pondrs::app::PondApp;
use pondrs::datasets::{MemoryDataset, Param};
use pondrs::error::PondError;
use pondrs::graph::build_pipeline_graph;
use pondrs::hooks::LoggingHook;
use pondrs::runners::{Runner, SequentialRunner, ParallelRunner};
use pondrs::{Dataset, Hooks, Node, Pipeline, Steps};

// ---------------------------------------------------------------------------
// Shared test types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct TestCatalog {
    a: MemoryDataset<i32>,
    b: MemoryDataset<i32>,
    c: MemoryDataset<i32>,
}

#[derive(Serialize, Deserialize)]
struct TestParams {
    scale: Param<i32>,
    offset: Param<i32>,
}

#[derive(Serialize, Deserialize)]
struct NestedParams {
    model: ModelParams,
    threshold: Param<f64>,
}

#[derive(Serialize, Deserialize)]
struct ModelParams {
    learning_rate: Param<f64>,
    epochs: Param<usize>,
}

/// Write YAML to a temp file and return its path string.
fn write_yaml(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path.to_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// Config loading tests
// ---------------------------------------------------------------------------

#[test]
fn test_load_yaml_and_deserialize_params() {
    let dir = TempDir::new().unwrap();
    let path = write_yaml(&dir, "params.yml", "scale: 5\noffset: 10\n");

    let value = load_yaml(&path).unwrap();
    let params: TestParams = deserialize_config(value).unwrap();
    assert_eq!(params.scale.0, 5);
    assert_eq!(params.offset.0, 10);
}

#[test]
fn test_load_yaml_and_deserialize_catalog() {
    let dir = TempDir::new().unwrap();
    // MemoryDataset skips its value field, so empty mapping works.
    let path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");

    let value = load_yaml(&path).unwrap();
    let _catalog: TestCatalog = deserialize_config(value).unwrap();
}

#[test]
fn test_load_yaml_missing_file() {
    let result = load_yaml("/nonexistent/path/missing.yml");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Override patching tests
// ---------------------------------------------------------------------------

#[test]
fn test_apply_overrides_flat_key() {
    let dir = TempDir::new().unwrap();
    let path = write_yaml(&dir, "params.yml", "scale: 5\noffset: 10\n");

    let mut value = load_yaml(&path).unwrap();
    apply_overrides(&mut value, &["scale=99".to_string()]);
    let params: TestParams = deserialize_config(value).unwrap();
    assert_eq!(params.scale.0, 99);
    assert_eq!(params.offset.0, 10); // unchanged
}

#[test]
fn test_apply_overrides_nested_dot_notation() {
    let dir = TempDir::new().unwrap();
    let path = write_yaml(
        &dir,
        "params.yml",
        "model:\n  learning_rate: 0.001\n  epochs: 10\nthreshold: 0.5\n",
    );

    let mut value = load_yaml(&path).unwrap();
    apply_overrides(
        &mut value,
        &[
            "model.learning_rate=0.01".to_string(),
            "model.epochs=50".to_string(),
            "threshold=0.9".to_string(),
        ],
    );

    let params: NestedParams = deserialize_config(value).unwrap();
    assert!((params.model.learning_rate.0 - 0.01).abs() < 1e-9);
    assert_eq!(params.model.epochs.0, 50);
    assert!((params.threshold.0 - 0.9).abs() < 1e-9);
}

#[test]
fn test_apply_overrides_bool_and_null_parsing() {
    let dir = TempDir::new().unwrap();
    let path = write_yaml(&dir, "conf.yml", "flag: false\ncount: 0\n");

    let mut value = load_yaml(&path).unwrap();
    apply_overrides(&mut value, &["flag=true".to_string(), "count=42".to_string()]);

    // Verify the raw YAML values were parsed correctly
    assert_eq!(value["flag"], serde_yaml::Value::Bool(true));
    assert_eq!(
        value["count"],
        serde_yaml::Value::Number(serde_yaml::Number::from(42))
    );
}

// ---------------------------------------------------------------------------
// PondApp: full pipeline run (sequential)
// ---------------------------------------------------------------------------

struct SeqApp;

impl PondApp for SeqApp {
    type Catalog = TestCatalog;
    type Params = TestParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(cat: &'a TestCatalog, params: &'a TestParams) -> Self::Pipeline<'a> {
        (
            Node {
                name: "multiply",
                func: |v: i32, scale: i32| (v * scale,),
                input: (&params.offset, &params.scale),
                output: (&cat.a,),
            },
            Node {
                name: "add",
                func: |a: i32, off: i32| (a + off,),
                input: (&cat.a, &params.offset),
                output: (&cat.b,),
            },
            Node {
                name: "square",
                func: |b: i32| (b * b,),
                input: (&cat.b,),
                output: (&cat.c,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        (LoggingHook::new(),)
    }

    fn runners() -> impl pondrs::Runners {
        (SequentialRunner,)
    }
}

#[test]
fn test_pond_app_run_sequential() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 3\noffset: 10\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = SeqApp::pipeline(&catalog, &params);
    let hooks = SeqApp::hooks();
    SequentialRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks).unwrap();

    // offset=10, scale=3 → a = 10*3 = 30, b = 30+10 = 40, c = 40*40 = 1600
    assert_eq!(catalog.c.load().unwrap(), 1600);
}

#[test]
fn test_pond_app_run_with_param_overrides() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 3\noffset: 10\n");

    let mut params_value = load_yaml(&params_path).unwrap();
    apply_overrides(&mut params_value, &["scale=5".to_string(), "offset=2".to_string()]);

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(params_value).unwrap();

    let pipeline = SeqApp::pipeline(&catalog, &params);
    let hooks = SeqApp::hooks();
    SequentialRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks).unwrap();

    // offset=2, scale=5 → a = 2*5 = 10, b = 10+2 = 12, c = 12*12 = 144
    assert_eq!(catalog.c.load().unwrap(), 144);
}

// ---------------------------------------------------------------------------
// PondApp: full pipeline run (parallel)
// ---------------------------------------------------------------------------

struct ParApp;

impl PondApp for ParApp {
    type Catalog = TestCatalog;
    type Params = TestParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(cat: &'a TestCatalog, params: &'a TestParams) -> Self::Pipeline<'a> {
        (
            // a and b can run in parallel (both read only from params)
            Node {
                name: "make_a",
                func: |v: i32| (v * 2,),
                input: (&params.scale,),
                output: (&cat.a,),
            },
            Node {
                name: "make_b",
                func: |v: i32| (v + 100,),
                input: (&params.offset,),
                output: (&cat.b,),
            },
            // c depends on both a and b
            Node {
                name: "combine",
                func: |a: i32, b: i32| (a + b,),
                input: (&cat.a, &cat.b),
                output: (&cat.c,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        ()
    }
}

#[test]
fn test_pond_app_run_parallel() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 7\noffset: 3\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = ParApp::pipeline(&catalog, &params);
    let hooks = ParApp::hooks();
    ParallelRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks).unwrap();

    // scale=7, offset=3 → a = 7*2 = 14, b = 3+100 = 103, c = 14+103 = 117
    assert_eq!(catalog.c.load().unwrap(), 117);
}

// ---------------------------------------------------------------------------
// PondApp: check on valid pipeline
// ---------------------------------------------------------------------------

#[test]
fn test_pond_app_check_valid() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 1\noffset: 1\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = SeqApp::pipeline(&catalog, &params);
    let graph = build_pipeline_graph(&pipeline, &catalog, &params);
    assert!(graph.check().is_ok());
    assert_eq!(graph.node_indices.len(), 3);
}

// ---------------------------------------------------------------------------
// PondApp: check on invalid pipeline (missing input)
// ---------------------------------------------------------------------------

struct BadApp;

impl PondApp for BadApp {
    type Catalog = TestCatalog;
    type Params = TestParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(cat: &'a TestCatalog, _params: &'a TestParams) -> Self::Pipeline<'a> {
        (
            // reads cat.a but nothing produces it (and it's not a param)
            Node {
                name: "bad_node",
                func: |a: i32| (a,),
                input: (&cat.a,),
                output: (&cat.b,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        ()
    }
}

#[test]
fn test_pond_app_check_invalid_missing_input() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 1\noffset: 1\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = BadApp::pipeline(&catalog, &params);
    let graph = build_pipeline_graph(&pipeline, &catalog, &params);
    let errors = graph.check().unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| {
        matches!(e, pondrs::graph::ValidationError::MissingInput { node_name: "bad_node", .. })
    }));
}

// ---------------------------------------------------------------------------
// PondApp: nested pipeline with check and run
// ---------------------------------------------------------------------------

struct NestedApp;

impl PondApp for NestedApp {
    type Catalog = TestCatalog;
    type Params = TestParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(cat: &'a TestCatalog, params: &'a TestParams) -> Self::Pipeline<'a> {
        (
            Node {
                name: "init",
                func: |s: i32| (s,),
                input: (&params.scale,),
                output: (&cat.a,),
            },
            Pipeline {
                name: "transform",
                steps: (
                    Node {
                        name: "add_offset",
                        func: |a: i32, off: i32| (a + off,),
                        input: (&cat.a, &params.offset),
                        output: (&cat.b,),
                    },
                    Node {
                        name: "double",
                        func: |b: i32| (b * 2,),
                        input: (&cat.b,),
                        output: (&cat.c,),
                    },
                ),
                input: (&cat.a, &params.offset),
                output: (&cat.c,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        (LoggingHook::new(),)
    }
}

#[test]
fn test_pond_app_nested_pipeline_check_and_run() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 4\noffset: 6\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = NestedApp::pipeline(&catalog, &params);

    // Check passes
    let graph = build_pipeline_graph(&pipeline, &catalog, &params);
    assert!(graph.check().is_ok());

    // Run and verify
    let hooks = NestedApp::hooks();
    SequentialRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks).unwrap();

    // scale=4, offset=6 → a=4, b=4+6=10, c=10*2=20
    assert_eq!(catalog.c.load().unwrap(), 20);
}

#[test]
fn test_pond_app_nested_pipeline_parallel() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 4\noffset: 6\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = NestedApp::pipeline(&catalog, &params);
    let hooks = NestedApp::hooks();
    ParallelRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks).unwrap();

    assert_eq!(catalog.c.load().unwrap(), 20);
}

// ---------------------------------------------------------------------------
// PondApp: error-returning node propagates through runner
// ---------------------------------------------------------------------------

struct ErrorApp;

impl PondApp for ErrorApp {
    type Catalog = TestCatalog;
    type Params = TestParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(cat: &'a TestCatalog, params: &'a TestParams) -> Self::Pipeline<'a> {
        (
            Node {
                name: "init",
                func: |v: i32| (v,),
                input: (&params.scale,),
                output: (&cat.a,),
            },
            Node {
                name: "fail",
                func: |_a: i32| -> Result<(i32,), PondError> {
                    Err(PondError::Custom("intentional failure".to_string()))
                },
                input: (&cat.a,),
                output: (&cat.b,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        ()
    }
}

#[test]
fn test_pond_app_error_propagation_sequential() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 1\noffset: 1\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = ErrorApp::pipeline(&catalog, &params);
    let hooks = ErrorApp::hooks();
    let result = SequentialRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("intentional failure"));
}

#[test]
fn test_pond_app_error_propagation_parallel() {
    let dir = TempDir::new().unwrap();
    let catalog_path = write_yaml(&dir, "catalog.yml", "a: {}\nb: {}\nc: {}\n");
    let params_path = write_yaml(&dir, "params.yml", "scale: 1\noffset: 1\n");

    let catalog: TestCatalog = deserialize_config(load_yaml(&catalog_path).unwrap()).unwrap();
    let params: TestParams = deserialize_config(load_yaml(&params_path).unwrap()).unwrap();

    let pipeline = ErrorApp::pipeline(&catalog, &params);
    let hooks = ErrorApp::hooks();
    let result = ParallelRunner.run::<PondError>(&pipeline, &catalog, &params, &hooks);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("intentional failure"));
}
