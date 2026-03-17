//! Dynamic pipeline example using [`StepVec`].
//!
//! Demonstrates runtime step composition: the `report` node is only included
//! when the `include_report` param is `true`.
//!
//! Usage:
//!   cargo run --example dyn_steps_app -- run
//!   cargo run --example dyn_steps_app -- check

#[path = "dyn_steps/mod.rs"]
mod dyn_steps;

use dyn_steps::{pipeline, write_fixtures};
use pondrs::error::PondError;

fn main() -> Result<(), PondError> {
    let dir = {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest.join("examples").join("dyn_steps_data")
    };
    write_fixtures(&dir);

    pondrs::app::App::from_yaml(
        dir.join("catalog.yml").to_str().unwrap(),
        dir.join("params.yml").to_str().unwrap(),
    )?
    .with_args(std::env::args_os())?
    .dispatch(pipeline)
}
