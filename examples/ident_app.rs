//! Example demonstrating the Ident node: write CSV as plain text, then read it
//! back as a Polars DataFrame via Ident, and produce a Plotly bar chart.
//!
//! Usage:
//!   cargo run --example ident_app -- --catalog-path examples/ident_data/catalog.yml \
//!       --params-path examples/ident_data/params.yml run

use plotly::{Bar, Layout, Plot};
use polars::prelude::*;
use serde::{Deserialize, Serialize};

use pondrs::datasets::{PlotlyDataset, PolarsCsvDataset, TextDataset};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;
use pondrs::{Ident, Node, Steps};

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct IdentCatalog {
    csv_text: TextDataset,
    csv_data: PolarsCsvDataset,
    chart: PlotlyDataset,
}

// ---------------------------------------------------------------------------
// Node functions
// ---------------------------------------------------------------------------

fn generate_csv() -> (String,) {
    let csv = "\
fruit,count
Apples,35
Bananas,22
Cherries,48
Dates,15
Elderberries,31
Figs,9
Grapes,42";
    (csv.to_string(),)
}

fn build_chart(df: DataFrame) -> (Plot,) {
    let fruits: Vec<String> = df
        .column("fruit")
        .unwrap()
        .str()
        .unwrap()
        .into_no_null_iter()
        .map(|s| s.to_string())
        .collect();
    let counts: Vec<i64> = df
        .column("count")
        .unwrap()
        .i64()
        .unwrap()
        .into_no_null_iter()
        .collect();

    let mut plot = Plot::new();
    plot.add_trace(Bar::new(fruits, counts).name("Fruit Count"));
    plot.set_layout(
        Layout::new()
            .title("Fruit Inventory")
            .y_axis(plotly::layout::Axis::new().title("Count")),
    );
    (plot,)
}

// ---------------------------------------------------------------------------
// Pipeline function
// ---------------------------------------------------------------------------

fn ident_pipeline<'a>(
    cat: &'a IdentCatalog,
    _params: &'a (),
) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "generate_csv",
            func: generate_csv,
            input: (),
            output: (&cat.csv_text,),
        },
        Ident {
            name: "text_to_csv",
            input: &cat.csv_text,
            output: &cat.csv_data,
        },
        Node {
            name: "build_chart",
            func: build_chart,
            input: (&cat.csv_data,),
            output: (&cat.chart,),
        },
    )
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), pondrs::error::PondError> {
    let dir = {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest.join("examples").join("ident_data")
    };

    std::fs::create_dir_all(&dir)?;

    std::fs::write(
        dir.join("catalog.yml"),
        format!(
            "\
csv_text:
  path: {d}/fruits.csv
csv_data:
  path: {d}/fruits.csv
chart:
  path: {d}/fruit_chart.json
",
            d = dir.display()
        ),
    )?;

    std::fs::write(dir.join("params.yml"), "~\n")?;

    let app = pondrs::app::App::from_args(std::env::args_os())?
        .with_hooks((
            LoggingHook::new(),
            VizHook::new("http://localhost:8080".to_string()),
        ));

    app.dispatch(ident_pipeline)?;
    Ok(())
}
