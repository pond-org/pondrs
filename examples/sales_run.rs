#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Example: monthly sales CSV → filter → total → Plotly bar chart,
//! using the PondApp trait interface.
//!
//! Pipeline (3 nodes):
//!   1. filter_months  — keep only months with sales ≥ min_sales param
//!   2. compute_total  — sum the filtered sales column
//!   3. build_chart    — produce a Plotly bar chart of filtered monthly sales
//!
//! Catalog and params are written as YAML to examples/data/, then the full
//! app entrypoint is invoked via `SalesApp::main_from(...)` with CLI args that
//! point at those files. The HTML chart path is printed at the end.
//!
//! Run alongside `sales_viz` to see live execution status in the pipeline
//! visualizer (start sales_viz first, then run this one).

#[path = "sales/mod.rs"]
mod sales;

use pondrs::app::PondApp;

use sales::{SalesApp, examples_data_dir, write_fixtures};

fn main() {
    let dir = examples_data_dir();
    write_fixtures(&dir);

    // Invoke the full app entrypoint, passing config paths as CLI args
    SalesApp::main_from([
        "sales-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "run",
    ]);

    println!("\nChart written to:");
    println!("  {}", dir.join("sales_chart.html").display());
    println!("Open with:  xdg-open \"{}\"", dir.join("sales_chart.html").display());
}
