//! Split/join pipeline example.
//!
//! Demonstrates TemplatedCatalog, Split, Join, and StepVec for fan-out/fan-in
//! patterns with per-item parallel processing.
//!
//! Usage:
//!   cargo run --example split_join_app -- run
//!   cargo run --example split_join_app -- check
//!   cargo run --example split_join_app -- viz

#[path = "split_join/mod.rs"]
mod split_join;

use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use split_join::{data_dir, pipeline, write_fixtures};

// ANCHOR: app
fn main() -> Result<(), PondError> {
    let dir = data_dir();
    write_fixtures(&dir);

    pondrs::app::App::from_yaml(
        dir.join("catalog.yml").to_str().unwrap(),
        dir.join("params.yml").to_str().unwrap(),
    )?
    .with_hooks((LoggingHook::new(),))
    .with_args(std::env::args_os())?
    .dispatch(pipeline)
}
// ANCHOR_END: app
