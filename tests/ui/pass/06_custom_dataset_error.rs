//! A dataset with its own error type, absorbed by a custom pipeline error type.
//!
//! `GpsError` has no `From` conversion into `PondError`. It reaches `AppError`
//! directly, keeping its type — the whole point of `E: From<D::Error>` on the
//! node input/output tuples.
use pondrs::datasets::{CellDataset, Param};
use pondrs::runners::{Runner, SequentialRunner};
use pondrs::{Dataset, Node, PondError};

#[derive(Debug)]
struct GpsError;

impl std::fmt::Display for GpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gps fix unavailable")
    }
}

impl std::error::Error for GpsError {}

#[derive(serde::Serialize)]
struct GpsDataset;

impl Dataset for GpsDataset {
    type LoadItem = f64;
    type SaveItem = f64;
    type Error = GpsError;
    fn load(&self) -> Result<f64, GpsError> {
        Ok(0.0)
    }
    fn save(&self, _output: f64) -> Result<(), GpsError> {
        Ok(())
    }
}

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

fn main() {
    let gps_in = GpsDataset;
    let gps_out = GpsDataset;
    let scale = Param(2.0f64);
    let summary = CellDataset::<f64>::new();

    let pipe = (
        // Mixes a custom-error dataset with built-in ones in one input tuple.
        Node {
            name: "scale",
            input: (&gps_in, &scale),
            output: (&gps_out,),
            func: |fix: f64, factor: f64| (fix * factor,),
        },
        Node {
            name: "summarize",
            input: (&gps_out,),
            output: (&summary,),
            func: |fix: f64| (fix,),
        },
    );

    let _: Result<(), AppError> = SequentialRunner.run(&pipe, &(), &(), &());
}
