#![feature(unboxed_closures, fn_traits, tuple_trait, impl_trait_in_assoc_type)]

//! Example: monthly sales CSV → filter → total → Plotly bar chart,
//! using the PondApp trait interface.
//!
//! Pipeline (3 nodes):
//!   1. filter_months  — keep only months with sales ≥ min_sales param
//!   2. compute_total  — sum the filtered sales column
//!   3. build_chart    — produce a Plotly bar chart of filtered monthly sales
//!
//! Catalog and params are written as YAML to a temp directory, then the full
//! app entrypoint is invoked via `SalesApp::main_from(...)` with CLI args that
//! point at those files. The HTML chart path is printed at the end.

use std::fs;

use plotly::{Bar, Layout, Plot};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use pondrs::app::PondApp;
use pondrs::datasets::{MemoryDataset, Param, PolarsCsvDataset, PlotlyDataset};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::{Hooks, Node, Steps};

// ---------------------------------------------------------------------------
// Catalog and params
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SalesCatalog {
    raw_sales: PolarsCsvDataset,
    filtered_sales: PolarsCsvDataset,
    total_sales: MemoryDataset<i64>,
    chart: PlotlyDataset,
}

#[derive(Serialize, Deserialize)]
struct SalesParams {
    /// Minimum sales to include a month in the chart.
    min_sales: Param<i64>,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct SalesApp;

impl PondApp for SalesApp {
    type Catalog = SalesCatalog;
    type Params = SalesParams;
    type Error = PondError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(cat: &'a SalesCatalog, params: &'a SalesParams) -> Self::Pipeline<'a> {
        (
            // Node 1: filter out months below the threshold
            Node {
                name: "filter_months",
                func: |df: DataFrame, min_sales: i64| -> Result<(DataFrame,), PolarsError> {
                    let mask = df.column("sales")?.i64()?.gt_eq(min_sales);
                    Ok((df.filter(&mask)?,))
                },
                input: (&cat.raw_sales, &params.min_sales),
                output: (&cat.filtered_sales,),
            },
            // Node 2: sum the filtered sales
            Node {
                name: "compute_total",
                func: |df: DataFrame| {
                    let total =
                        df.column("sales").unwrap().i64().unwrap().sum().unwrap_or(0);
                    (total,)
                },
                input: (&cat.filtered_sales,),
                output: (&cat.total_sales,),
            },
            // Node 3: build a bar chart of the filtered monthly sales
            Node {
                name: "build_chart",
                func: |df: DataFrame, total: i64| {
                    let months: Vec<String> = df
                        .column("month").unwrap()
                        .str().unwrap()
                        .into_no_null_iter()
                        .map(|s| s.to_string())
                        .collect();
                    let sales: Vec<i64> = df
                        .column("sales").unwrap()
                        .i64().unwrap()
                        .into_no_null_iter()
                        .collect();

                    let mut plot = Plot::new();
                    plot.add_trace(Bar::new(months, sales).name("Monthly Sales"));
                    plot.set_layout(
                        Layout::new()
                            .title(format!("Months with sales ≥ 1000  (total: {total})"))
                            .y_axis(plotly::layout::Axis::new().title("Sales")),
                    );
                    (plot,)
                },
                input: (&cat.filtered_sales, &cat.total_sales),
                output: (&cat.chart,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        (LoggingHook::new(),)
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();

    // Write input CSV
    fs::write(
        d.join("monthly_sales.csv"),
        "\
month,sales
Jan,1200
Feb,800
Mar,1500
Apr,600
May,1800
Jun,950
Jul,2100
Aug,700
Sep,1600
Oct,1300
Nov,400
Dec,2200
",
    )
    .unwrap();

    // Write catalog YAML — paths point into the temp dir
    fs::write(
        d.join("catalog.yml"),
        format!(
            "\
raw_sales:
  path: {d}/monthly_sales.csv
filtered_sales:
  path: {d}/filtered_sales.csv
total_sales: {{}}
chart:
  path: {d}/sales_chart.json
",
            d = d.display()
        ),
    )
    .unwrap();

    // Write params YAML
    fs::write(d.join("params.yml"), "min_sales: 1000\n").unwrap();

    // Determine the output path before handing control to main_from
    let html_path = d.join("sales_chart.html");

    // Invoke the full app entrypoint, passing config paths as CLI args
    SalesApp::main_from([
        "sales-app",
        "--catalog-path", d.join("catalog.yml").to_str().unwrap(),
        "--params-path",  d.join("params.yml").to_str().unwrap(),
        "run",
    ]);

    // main_from returned, so the pipeline succeeded
    let dir_path = dir.keep();
    println!("\nChart written to:");
    println!("  {}", dir_path.join("sales_chart.html").display());
    println!("Open with:  xdg-open \"{}\"", html_path.display());
}
