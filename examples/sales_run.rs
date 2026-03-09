//! Example: monthly sales CSV → filter → total → Plotly bar chart.
//!
//! Pipeline (3 nodes):
//!   1. filter_months  — keep only months with sales ≥ min_sales param
//!   2. compute_total  — sum the filtered sales column
//!   3. build_chart    — produce a Plotly bar chart of filtered monthly sales
//!
//! Run alongside `sales_viz` to see live execution status in the pipeline
//! visualizer (start sales_viz first, then run this one).

#[path = "sales/mod.rs"]
mod sales;

use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;

use sales::{sales_pipeline, examples_data_dir, write_fixtures};

fn main() -> Result<(), pondrs::error::PondError> {
    let dir = examples_data_dir();
    write_fixtures(&dir);

    let app = pondrs::app::App::from_args([
        "sales-app",
        "--catalog-path", dir.join("catalog.yml").to_str().unwrap(),
        "--params-path",  dir.join("params.yml").to_str().unwrap(),
        "run",
    ])?
    .with_hooks((
        LoggingHook::new(),
        VizHook::new("http://localhost:8080".to_string()),
    ));

    app.dispatch(sales_pipeline)?;

    println!("\nChart written to:");
    println!("  {}", dir.join("sales_chart.html").display());
    println!("Open with:  xdg-open \"{}\"", dir.join("sales_chart.html").display());
    Ok(())
}
