//! Sales pipeline app.
//!
//! Usage:
//!   cargo run --example sales_app -- run
//!   cargo run --example sales_app -- check
//!   cargo run --example sales_app -- viz

#[path = "sales/mod.rs"]
mod sales;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use sales::{examples_data_dir, sales_pipeline, write_fixtures};

fn main() -> Result<(), pondrs::error::PondError> {
    let dir = examples_data_dir();
    write_fixtures(&dir);

    // ANCHOR: app
    pondrs::app::App::from_yaml(
        dir.join("catalog.yml").to_str().unwrap(),
        dir.join("params.yml").to_str().unwrap(),
    )?
    .with_hooks((
        LoggingHook::new(),
        VizHook::new("http://localhost:8080".to_string()),
    ))
    .with_args(std::env::args_os())?
    .dispatch(sales_pipeline)
    // ANCHOR_END: app
}
