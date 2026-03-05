#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Weather station analysis example — interactive pipeline visualizer.
//!
//! Starts the viz web server on port 8080. Open http://localhost:8080 in your
//! browser to inspect the pipeline graph. Then run `weather_run` in a second
//! terminal to see live execution status (including the intentional error).
//!
//! Usage:
//!   Terminal 1:  cargo run --example weather_viz
//!   Browser:     http://localhost:8080
//!   Terminal 2:  cargo run --example weather_run

#[path = "weather/mod.rs"]
mod weather;

use pondrs::app::PondApp;

use weather::{WeatherApp, weather_data_dir, write_fixtures};

fn main() {
    let dir = weather_data_dir();
    write_fixtures(&dir);

    println!("Starting viz server on http://localhost:8080");
    println!("Open that URL in your browser, then in a second terminal run:");
    println!("  cargo run --example weather_run");
    println!("to see live execution status (including the intentional error).\n");
    println!("Press Ctrl+C to stop.");

    WeatherApp::main_from([
        "weather-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "viz",
        "--port", "8080",
    ]);
}
