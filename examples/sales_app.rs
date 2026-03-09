//! Thin CLI wrapper for the sales pipeline app.
//!
//! Writes fixtures to examples/data/ and then dispatches based on CLI args.
//!
//! Usage:
//!   cargo run --example sales_app -- --catalog-path examples/data/catalog.yml \
//!       --params-path examples/data/params.yml run
//!   cargo run --example sales_app -- --catalog-path examples/data/catalog.yml \
//!       --params-path examples/data/params.yml check
//!   cargo run --example sales_app -- --catalog-path examples/data/catalog.yml \
//!       --params-path examples/data/params.yml viz

#[path = "sales/mod.rs"]
mod sales;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use sales::{sales_pipeline, examples_data_dir, write_fixtures};

fn main() -> Result<(), pondrs::error::PondError> {
    write_fixtures(&examples_data_dir());

    let app = pondrs::app::App::from_args(std::env::args_os())?
        .with_hooks((
            LoggingHook::new(),
            VizHook::new("http://localhost:8080".to_string()),
        ));

    app.dispatch(sales_pipeline)?;
    Ok(())
}
