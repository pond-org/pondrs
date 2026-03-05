#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

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

use pondrs::app::PondApp;

use sales::{SalesApp, examples_data_dir, write_fixtures};

fn main() {
    let dir = examples_data_dir();
    write_fixtures(&dir);

    println!("Starting viz server on http://localhost:8080");
    println!("Open that URL in your browser, then in a second terminal run:");
    println!("  cargo run --example sales_run");
    println!("to see live execution status stream in.\n");
    println!("Press Ctrl+C to stop.");

    // Launch the viz subcommand — blocks until Ctrl+C
    SalesApp::main_from([
        "sales-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "viz",
        "--port", "8080",
    ]);
}
