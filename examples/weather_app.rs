//! Weather pipeline app.
//!
//! Usage:
//!   cargo run --example weather_app -- run --runner parallel
//!   cargo run --example weather_app -- check
//!   cargo run --example weather_app -- viz

#[path = "weather/mod.rs"]
mod weather;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use weather::{WeatherError, weather_data_dir, weather_pipeline, write_fixtures};

fn main() -> Result<(), WeatherError> {
    let dir = weather_data_dir();
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
    .dispatch(weather_pipeline)
    // ANCHOR_END: app
}
