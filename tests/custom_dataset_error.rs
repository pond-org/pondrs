//! A dataset error type that has no conversion into `PondError` still reaches
//! the pipeline error type, with its type intact.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use pondrs::datasets::{
    CellDataset, MemoryDataset, Never, Param, PartitionedDataset, TemplatedCatalog, TextDataset,
};
use pondrs::{Dataset, EachField, Node, PartitionedNode, PondError, Runner, SequentialRunner};

// --- A dataset with a domain error that PondError knows nothing about -------

#[derive(Debug, PartialEq)]
enum GpsError {
    NoFix { satellites: u8 },
    Garbled,
}

impl std::fmt::Display for GpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpsError::NoFix { satellites } => write!(f, "no fix ({satellites} satellites)"),
            GpsError::Garbled => write!(f, "garbled sentence"),
        }
    }
}

impl std::error::Error for GpsError {}

#[derive(Serialize, Deserialize)]
struct GpsDataset {
    /// `None` means "loading fails"; a value means "loading succeeds with it".
    fix: Option<f64>,
    #[serde(skip)]
    saved: std::sync::Mutex<Option<f64>>,
}

impl GpsDataset {
    fn with_fix(fix: f64) -> Self {
        Self { fix: Some(fix), saved: std::sync::Mutex::new(None) }
    }
    fn broken() -> Self {
        Self { fix: None, saved: std::sync::Mutex::new(None) }
    }
    fn sink() -> Self {
        Self { fix: Some(0.0), saved: std::sync::Mutex::new(None) }
    }
}

impl Dataset for GpsDataset {
    type LoadItem = f64;
    type SaveItem = f64;
    type Error = GpsError;

    fn load(&self) -> Result<f64, GpsError> {
        self.fix.ok_or(GpsError::NoFix { satellites: 2 })
    }

    fn save(&self, output: f64) -> Result<(), GpsError> {
        if output.is_nan() {
            return Err(GpsError::Garbled);
        }
        *self.saved.lock().unwrap() = Some(output);
        Ok(())
    }
}

// --- The pipeline error type the user writes -------------------------------

#[derive(Debug)]
enum AppError {
    Pond(PondError),
    Gps(GpsError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Pond(e) => write!(f, "{e}"),
            AppError::Gps(e) => write!(f, "{e}"),
        }
    }
}

impl From<PondError> for AppError {
    fn from(e: PondError) -> Self {
        AppError::Pond(e)
    }
}

impl From<GpsError> for AppError {
    fn from(e: GpsError) -> Self {
        AppError::Gps(e)
    }
}

// --- Tests -----------------------------------------------------------------

#[test]
fn custom_dataset_error_flows_into_pipeline_error() {
    let gps = GpsDataset::broken();
    let out = CellDataset::<f64>::new();

    let pipe = (Node {
        name: "read_gps",
        input: (&gps,),
        output: (&out,),
        func: |fix: f64| (fix,),
    },);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());

    // The point of the whole change: `GpsError` arrives as `GpsError`, not
    // stringified through `PondError::Custom`.
    match result {
        Err(AppError::Gps(GpsError::NoFix { satellites })) => assert_eq!(satellites, 2),
        other => panic!("expected AppError::Gps(NoFix), got {other:?}"),
    }
}

#[test]
fn custom_dataset_save_error_flows_into_pipeline_error() {
    let gps = GpsDataset::sink();
    let source = Param(f64::NAN);

    let pipe = (Node {
        name: "write_gps",
        input: (&source,),
        output: (&gps,),
        func: |v: f64| (v,),
    },);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(matches!(result, Err(AppError::Gps(GpsError::Garbled))), "got {result:?}");
}

#[test]
fn custom_and_builtin_dataset_errors_mix_in_one_tuple() {
    let gps = GpsDataset::with_fix(3.0);
    let factor = Param(2.0f64);
    let out = MemoryDataset::<f64>::new();

    // `&GpsDataset` (Error = GpsError) and `&Param` / `&MemoryDataset`
    // (Error = PondError) in the same input tuple: `AppError` absorbs both.
    let pipe = (Node {
        name: "scale",
        input: (&gps, &factor),
        output: (&out,),
        func: |fix: f64, f: f64| (fix * f,),
    },);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(result.is_ok(), "got {result:?}");
    assert_eq!(out.load().unwrap(), 6.0);
}

#[test]
fn framework_errors_still_land_in_the_pond_variant() {
    // `MemoryDataset` was never written, so loading it is a framework error.
    let empty = MemoryDataset::<f64>::new();
    let out = CellDataset::<f64>::new();

    let pipe = (Node {
        name: "read_empty",
        input: (&empty,),
        output: (&out,),
        func: |v: f64| (v,),
    },);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(
        matches!(result, Err(AppError::Pond(PondError::DatasetNotLoaded))),
        "got {result:?}"
    );
}

// --- EachField carries the dataset error through too ------------------------

#[derive(Serialize, Deserialize)]
struct GpsEntry {
    reading: GpsDataset,
}

fn each_field_catalog(yaml: &str) -> TemplatedCatalog<GpsEntry> {
    serde_yaml::from_str(yaml).unwrap()
}

#[test]
fn each_field_propagates_dataset_error() {
    let catalog = each_field_catalog(
        r#"
template:
  reading: { fix: null }
names: [bow, stern]
"#,
    );
    let out = MemoryDataset::<HashMap<String, f64>>::new();

    let pipe = (Node {
        name: "join",
        input: (EachField { catalog: &catalog, field: |e: &GpsEntry| &e.reading },),
        output: (&out,),
        func: |m: HashMap<String, f64>| (m,),
    },);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(
        matches!(result, Err(AppError::Gps(GpsError::NoFix { .. }))),
        "got {result:?}"
    );
}

#[test]
fn each_field_key_mismatch_still_lands_in_the_pond_variant() {
    let catalog = each_field_catalog(
        r#"
template:
  reading: { fix: 1.0 }
names: [bow, stern]
"#,
    );
    let source = Param(vec!["bow".to_string(), "midships".to_string()]);

    // `KeyMismatch` is EachField's own error, not the dataset's — it has no home
    // in `Self::Error = D::Error`, which is why the `From<PondError>` floor stays.
    let pipe = (Node {
        name: "split",
        input: (&source,),
        output: (EachField { catalog: &catalog, field: |e: &GpsEntry| &e.reading },),
        func: |keys: Vec<String>| (keys.into_iter().map(|k| (k, 1.0)).collect::<HashMap<_, _>>(),),
    },);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(
        matches!(result, Err(AppError::Pond(PondError::KeyMismatch { .. }))),
        "got {result:?}"
    );
}

// --- PartitionedNode functions may return a custom error too -----------------

#[derive(Serialize)]
struct PartCatalog {
    input: PartitionedDataset<TextDataset>,
    output: PartitionedDataset<TextDataset>,
}

fn shout(text: String) -> Result<(String,), GpsError> {
    if text.contains("bad") {
        return Err(GpsError::Garbled);
    }
    Ok((text.to_uppercase(),))
}

fn part_catalog(dir: &std::path::Path, input: &str) -> PartCatalog {
    let in_dir = dir.join("in");
    std::fs::create_dir_all(&in_dir).unwrap();
    std::fs::write(in_dir.join("a.txt"), input).unwrap();
    PartCatalog {
        input: PartitionedDataset {
            path: in_dir.to_str().unwrap().into(),
            ext: "txt".into(),
            dataset: TextDataset::new(""),
        },
        output: PartitionedDataset {
            path: dir.join("out").to_str().unwrap().into(),
            ext: "txt".into(),
            dataset: TextDataset::new(""),
        },
    }
}

#[test]
fn partitioned_node_function_error_flows_into_pipeline_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let cat = part_catalog(dir.path(), "bad input");

    // Before this change `PartitionedNode` pinned its function to
    // `IntoNodeResult<_, PondError>`, so `shout` could not be used at all.
    let pipe = (PartitionedNode::new("shout", shout, &cat.input, &cat.output),);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(matches!(result, Err(AppError::Gps(GpsError::Garbled))), "got {result:?}");
}

#[test]
fn partitioned_node_with_custom_error_succeeds() {
    let dir = tempfile::TempDir::new().unwrap();
    let cat = part_catalog(dir.path(), "good input");

    let pipe = (PartitionedNode::new("shout", shout, &cat.input, &cat.output),);

    let result: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
    assert!(result.is_ok(), "got {result:?}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("out/a.txt")).unwrap(),
        "GOOD INPUT"
    );
}

// `Never` is still uninhabited: a `&Param` in an output tuple remains a compile
// error. Referenced here so the import is used and the intent is recorded.
#[allow(dead_code)]
fn never_is_still_uninhabited(n: Never) -> ! {
    match n {}
}
