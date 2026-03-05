#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Weather station analysis example — run with the parallel runner.
//!
//! Demonstrates: subpipelines, struct params, nested catalog/params,
//! PartitionedDataset, MemoryDataset, YamlDataset, PlotlyDataset,
//! parallel execution, and an intentional error node.
//!
//! Usage:
//!   Terminal 1 (optional):  cargo run --example weather_viz
//!   Terminal 2:             cargo run --example weather_run

#[path = "weather/mod.rs"]
mod weather;

use pondrs::app::PondApp;

use weather::{WeatherApp, weather_data_dir, write_fixtures};

fn main() {
    let dir = weather_data_dir();
    write_fixtures(&dir);

    println!("Running weather pipeline with parallel runner...");
    println!("(The validate_reports node will intentionally fail)\n");

    WeatherApp::main_from([
        "weather-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "run",
        "--runner", "parallel",
    ]);
}
