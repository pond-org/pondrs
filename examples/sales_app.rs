#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Thin CLI wrapper for the sales pipeline app.
//!
//! Writes fixtures to examples/data/ and then delegates to `SalesApp::main()`,
//! which reads subcommands and flags from the command line.
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

use pondrs::app::PondApp;

use sales::{SalesApp, examples_data_dir, write_fixtures};

fn main() {
    write_fixtures(&examples_data_dir());
    SalesApp::main();
}
