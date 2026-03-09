//! Weather station analysis example — interactive pipeline visualizer.
//!
//! Usage:
//!   Terminal 1:  cargo run --example weather_viz
//!   Browser:     http://localhost:8080
//!   Terminal 2:  cargo run --example weather_run

#[path = "weather/mod.rs"]
mod weather;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use weather::{WeatherError, weather_pipeline, weather_data_dir, write_fixtures};

fn main() -> Result<(), WeatherError> {
    let dir = weather_data_dir();
    write_fixtures(&dir);

    println!("Starting viz server on http://localhost:8080");
    println!("Open that URL in your browser, then in a second terminal run:");
    println!("  cargo run --example weather_run");
    println!("to see live execution status (including the intentional error).\n");
    println!("Press Ctrl+C to stop.");

    let app = pondrs::app::App::from_args([
        "weather-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "viz",
        "--port", "8080",
    ])?
    .with_hooks((
        LoggingHook::new(),
        VizHook::new("http://localhost:8080".to_string()),
    ));

    app.dispatch(weather_pipeline)?;
    Ok(())
}
