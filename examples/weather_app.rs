#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Thin CLI wrapper for the weather pipeline app.
//!
//! Writes fixtures to examples/weather_data/ and then delegates to
//! `WeatherApp::main()`, which reads subcommands and flags from the
//! command line.
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

use pondrs::app::PondApp;

use weather::{WeatherApp, weather_data_dir, write_fixtures};

fn main() {
    write_fixtures(&weather_data_dir());
    WeatherApp::main();
}
