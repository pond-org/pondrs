//! Example: launch the interactive pipeline visualizer for the sales pipeline.
//!
//! Starts the viz web server on port 8080. Open http://localhost:8080 in your
//! browser to inspect the pipeline graph. Then run `sales_run` in a
//! second terminal to see live execution status stream in via VizHook.
//!
//! Usage:
//!   Terminal 1:  cargo run --example sales_viz
//!   Browser:     http://localhost:8080
//!   Terminal 2:  cargo run --example sales_run

#[path = "sales/mod.rs"]
mod sales;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use sales::{sales_pipeline, examples_data_dir, write_fixtures};

fn main() -> Result<(), pondrs::error::PondError> {
    let dir = examples_data_dir();
    write_fixtures(&dir);

    println!("Starting viz server on http://localhost:8080");
    println!("Open that URL in your browser, then in a second terminal run:");
    println!("  cargo run --example sales_run");
    println!("to see live execution status stream in.\n");
    println!("Press Ctrl+C to stop.");

    let app = pondrs::app::App::from_args([
        "sales-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "viz",
        "--port", "8080",
    ])?
    .with_hooks((
        LoggingHook::new(),
        VizHook::new("http://localhost:8080".to_string()),
    ));

    app.dispatch(sales_pipeline)?;
    Ok(())
}
