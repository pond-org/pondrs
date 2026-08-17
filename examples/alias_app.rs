//! Example demonstrating the Alias node: write CSV as plain text, then read it
//! back as a Polars `DataFrame` via Alias, and produce a Plotly bar chart.
//!
//! Usage:
//!   cargo run --example `alias_app` -- --catalog-path `examples/alias_data/catalog.yml` \
//!       --params-path `examples/alias_data/params.yml` run

use plotly::{Bar, Layout, Plot};
use polars::prelude::*;
use serde::{Deserialize, Serialize};

use pondrs::datasets::{PlotlyDataset, PolarsCsvDataset, TextDataset};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;
use pondrs::{Alias, Node, Steps};

// ANCHOR: types
// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct AliasCatalog {
    csv_text: TextDataset,
    csv_data: PolarsCsvDataset,
    chart: PlotlyDataset,
}

// ANCHOR_END: types

// ANCHOR: nodes
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
        .map(ToString::to_string)
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

// ANCHOR_END: nodes

// ANCHOR: pipeline
// ---------------------------------------------------------------------------
// Pipeline function
// ---------------------------------------------------------------------------

fn alias_pipeline<'a>(
    cat: &'a AliasCatalog,
    _params: &'a (),
) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "generate_csv",
            input: (),
            output: (&cat.csv_text,),
            func: generate_csv,
        },
        Alias {
            name: "text_to_csv",
            input: &cat.csv_text,
            output: &cat.csv_data,
        },
        Node {
            name: "build_chart",
            input: (&cat.csv_data,),
            output: (&cat.chart,),
            func: build_chart,
        },
    )
}

// ANCHOR_END: pipeline

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), pondrs::error::PondError> {
    let dir = {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest.join("examples").join("alias_data")
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

    app.dispatch(alias_pipeline)?;
    Ok(())
}
