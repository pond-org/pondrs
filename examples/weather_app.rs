//! Thin CLI wrapper for the weather pipeline app.
//!
//! Usage:
//!   cargo run --example weather_app -- --catalog-path examples/weather_data/catalog.yml \
//!       --params-path examples/weather_data/params.yml run --runner parallel
//!   cargo run --example weather_app -- --catalog-path examples/weather_data/catalog.yml \
//!       --params-path examples/weather_data/params.yml check
//!   cargo run --example weather_app -- --catalog-path examples/weather_data/catalog.yml \
//!       --params-path examples/weather_data/params.yml viz

#[path = "weather/mod.rs"]
mod weather;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use weather::{WeatherError, weather_pipeline, weather_data_dir, write_fixtures};

fn main() -> Result<(), WeatherError> {
    write_fixtures(&weather_data_dir());

    let app = pondrs::app::App::from_args(std::env::args_os())?
        .with_hooks((
            LoggingHook::new(),
            VizHook::new("http://localhost:8080".to_string()),
        ));

    app.dispatch(weather_pipeline)?;
    Ok(())
}
