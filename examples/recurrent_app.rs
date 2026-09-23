//! Recurrent pipeline example.
//!
//! Demonstrates `RecurrentNode`: a loop unrolled into one node per iteration
//! (`train/0` … `train/4`), each writing its own checkpoint. Because every
//! iteration is a real node, it can be addressed on its own:
//!
//! Usage:
//!   cargo run --example `recurrent_app` -- run
//!   cargo run --example `recurrent_app` -- run --from-nodes train/3
//!   cargo run --example `recurrent_app` -- run --catalog checkpoints.count=20
//!   cargo run --example `recurrent_app` -- check
//!   cargo run --example `recurrent_app` -- viz

#[path = "recurrent/mod.rs"]
mod recurrent;

use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;
use recurrent::{data_dir, pipeline, write_fixtures};

// ANCHOR: app
fn main() -> Result<(), PondError> {
    let dir = data_dir();
    write_fixtures(&dir);

    pondrs::app::App::from_yaml(
        dir.join("catalog.yml").to_str().unwrap(),
        dir.join("params.yml").to_str().unwrap(),
    )?
    .with_hooks((
        LoggingHook::new(),
        VizHook::new("http://localhost:8080".to_string()),
    ))
    .with_args(std::env::args_os())?
    .dispatch(pipeline)
}
// ANCHOR_END: app
