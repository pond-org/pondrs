//! Shared pipeline definition for the weather station analysis example.
//!
//! Demonstrates: subpipelines, struct params, nested catalog/params,
//! PartitionedDataset, MemoryDataset, YamlDataset, PlotlyDataset,
//! parallel nodes, and an intentional error node.

use std::collections::HashMap;

use plotly::{Bar, Layout, Plot};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use yaml_rust2::Yaml;

use pondrs::app::PondApp;
use pondrs::datasets::{
    MemoryDataset, Param, PartitionedDataset, PlotlyDataset, PolarsCsvDataset, YamlDataset,
};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::viz::VizHook;
use pondrs::{Hooks, Node, Pipeline, Steps};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum WeatherError {
    Pond(PondError),
    Polars(PolarsError),
    Validation(String),
}

impl From<PondError> for WeatherError {
    fn from(e: PondError) -> Self {
        WeatherError::Pond(e)
    }
}

impl From<PolarsError> for WeatherError {
    fn from(e: PolarsError) -> Self {
        WeatherError::Polars(e)
    }
}

impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeatherError::Pond(e) => write!(f, "{e}"),
            WeatherError::Polars(e) => write!(f, "{e}"),
            WeatherError::Validation(msg) => write!(f, "Validation failed: {msg}"),
        }
    }
}

impl std::error::Error for WeatherError {}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Baseline period for anomaly detection (struct param).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BaselinePeriod {
    pub start_month: u32,
    pub end_month: u32,
}

/// Aggregated weather statistics (must be Copy for MemoryDataset).
#[derive(Clone, Copy, Debug, Default)]
pub struct WeatherSummary {
    pub avg_temp: f64,
    pub max_temp: f64,
    pub min_temp: f64,
    pub total_rainfall: f64,
    pub station_count: u32,
    pub reading_count: u32,
}

// ---------------------------------------------------------------------------
// Params (nested containers with struct param)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct WeatherParams {
    pub analysis: AnalysisConfig,
    pub display: DisplayConfig,
}

#[derive(Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub anomaly_threshold: Param<f64>,
    pub baseline: Param<BaselinePeriod>,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayConfig {
    pub chart_title: Param<String>,
    pub top_n: Param<usize>,
}

// ---------------------------------------------------------------------------
// Catalog (nested containers)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct WeatherCatalog {
    pub sources: SourcesCatalog,
    pub analysis: AnalysisCatalog,
    pub reports: ReportsCatalog,
}

#[derive(Serialize, Deserialize)]
pub struct SourcesCatalog {
    pub station_readings: PartitionedDataset<PolarsCsvDataset>,
    pub station_metadata: YamlDataset,
}

#[derive(Serialize, Deserialize)]
pub struct AnalysisCatalog {
    pub combined_readings: PolarsCsvDataset,
    pub weather_summary: MemoryDataset<WeatherSummary>,
    pub anomalies: PolarsCsvDataset,
}

#[derive(Serialize, Deserialize)]
pub struct ReportsCatalog {
    pub temperature_chart: PlotlyDataset,
    pub rainfall_chart: PlotlyDataset,
    pub validation_passed: MemoryDataset<bool>,
}

// ---------------------------------------------------------------------------
// Node functions
// ---------------------------------------------------------------------------

/// Merge all station CSV partitions into a single DataFrame, adding a
/// "station" column from the partition key.
fn merge_stations(
    partitions: HashMap<String, DataFrame>,
) -> Result<(DataFrame,), PolarsError> {
    let mut combined: Option<DataFrame> = None;
    for (station_name, mut df) in partitions {
        let n = df.height();
        let station_col = Column::new("station".into(), vec![station_name.as_str(); n]);
        df.with_column(station_col)?;
        match &mut combined {
            None => combined = Some(df),
            Some(c) => { c.vstack_mut(&df)?; }
        }
    }
    Ok((combined.unwrap_or_default(),))
}

/// Load station metadata from YAML (side-effect node, just prints info).
fn load_metadata(meta: Yaml) {
    if let Yaml::Hash(ref map) = meta {
        for (key, _) in map {
            if let Yaml::String(name) = key {
                log::info!("Loaded metadata for station: {name}");
            }
        }
    }
}

/// Compute aggregate weather statistics from the combined readings.
fn compute_summary(
    df: DataFrame,
    baseline: BaselinePeriod,
) -> (WeatherSummary,) {
    let _ = baseline; // used for filtering in a real scenario

    let temp = df.column("temperature").unwrap().f64().unwrap();
    let rain = df.column("rainfall").unwrap().f64().unwrap();

    let stations: Vec<String> = df
        .column("station")
        .unwrap()
        .str()
        .unwrap()
        .into_no_null_iter()
        .map(|s| s.to_string())
        .collect();
    let unique_stations: std::collections::HashSet<&str> =
        stations.iter().map(|s| s.as_str()).collect();

    let summary = WeatherSummary {
        avg_temp: temp.mean().unwrap_or(0.0),
        max_temp: temp.max().unwrap_or(0.0),
        min_temp: temp.min().unwrap_or(0.0),
        total_rainfall: rain.sum().unwrap_or(0.0),
        station_count: unique_stations.len() as u32,
        reading_count: df.height() as u32,
    };
    (summary,)
}

/// Detect anomalous temperature readings (beyond threshold * std_dev from mean).
fn detect_anomalies(
    df: DataFrame,
    threshold: f64,
) -> Result<(DataFrame,), PolarsError> {
    let temp = df.column("temperature")?.f64()?;
    let mean = temp.mean().unwrap_or(0.0);
    let std_dev = temp.std(1).unwrap_or(1.0);

    let lower = mean - threshold * std_dev;
    let upper = mean + threshold * std_dev;

    let mask = temp.lt(lower) | temp.gt(upper);
    Ok((df.filter(&mask)?,))
}

/// Build a temperature bar chart from the summary.
fn plot_temperatures(summary: WeatherSummary, title: String) -> (Plot,) {
    let labels = vec!["Min", "Avg", "Max"];
    let values = vec![summary.min_temp, summary.avg_temp, summary.max_temp];

    let mut plot = Plot::new();
    plot.add_trace(
        Bar::new(labels, values).name("Temperature"),
    );
    plot.set_layout(
        Layout::new()
            .title(format!("{title} - Temperature Summary"))
            .y_axis(plotly::layout::Axis::new().title("Temperature (C)")),
    );
    (plot,)
}

/// Build a rainfall summary chart.
fn plot_rainfall(summary: WeatherSummary, title: String) -> (Plot,) {
    let labels = vec![
        format!("{} stations", summary.station_count),
        format!("{} readings", summary.reading_count),
    ];
    let values = vec![
        summary.total_rainfall / summary.station_count as f64,
        summary.total_rainfall,
    ];

    let mut plot = Plot::new();
    plot.add_trace(
        Bar::new(labels, values).name("Rainfall"),
    );
    plot.set_layout(
        Layout::new()
            .title(format!("{title} - Rainfall Summary"))
            .y_axis(plotly::layout::Axis::new().title("Rainfall (mm)")),
    );
    (plot,)
}

/// Validation node that always fails (demonstrates error state in viz).
fn validate_reports(
    _temp_chart: serde_json::Value,
    _rain_chart: serde_json::Value,
) -> Result<(bool,), WeatherError> {
    Err(WeatherError::Validation(
        "Data quality check failed: east station has a suspicious 50C reading in July".into(),
    ))
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct WeatherApp;

impl PondApp for WeatherApp {
    type Catalog = WeatherCatalog;
    type Params = WeatherParams;
    type Error = WeatherError;
    type Pipeline<'a> = impl Steps<Self::Error>;

    fn pipeline<'a>(
        cat: &'a WeatherCatalog,
        params: &'a WeatherParams,
    ) -> Self::Pipeline<'a> {
        (
            // Subpipeline: data preparation
            Pipeline {
                name: "data_prep",
                steps: (
                    Node {
                        name: "merge_stations",
                        func: merge_stations,
                        input: (&cat.sources.station_readings,),
                        output: (&cat.analysis.combined_readings,),
                    },
                    Node {
                        name: "load_metadata",
                        func: load_metadata,
                        input: (&cat.sources.station_metadata,),
                        output: (),
                    },
                ),
                input: (
                    &cat.sources.station_readings,
                    &cat.sources.station_metadata,
                ),
                output: (&cat.analysis.combined_readings,),
            },
            // These two nodes can run in parallel (both read combined_readings)
            Node {
                name: "compute_summary",
                func: compute_summary,
                input: (
                    &cat.analysis.combined_readings,
                    &params.analysis.baseline,
                ),
                output: (&cat.analysis.weather_summary,),
            },
            Node {
                name: "detect_anomalies",
                func: detect_anomalies,
                input: (
                    &cat.analysis.combined_readings,
                    &params.analysis.anomaly_threshold,
                ),
                output: (&cat.analysis.anomalies,),
            },
            // Subpipeline: reporting (inner nodes can run in parallel)
            Pipeline {
                name: "reporting",
                steps: (
                    Node {
                        name: "plot_temperatures",
                        func: plot_temperatures,
                        input: (
                            &cat.analysis.weather_summary,
                            &params.display.chart_title,
                        ),
                        output: (&cat.reports.temperature_chart,),
                    },
                    Node {
                        name: "plot_rainfall",
                        func: plot_rainfall,
                        input: (
                            &cat.analysis.weather_summary,
                            &params.display.chart_title,
                        ),
                        output: (&cat.reports.rainfall_chart,),
                    },
                ),
                input: (
                    &cat.analysis.weather_summary,
                    &params.display.chart_title,
                ),
                output: (
                    &cat.reports.temperature_chart,
                    &cat.reports.rainfall_chart,
                ),
            },
            // Validation node: intentionally fails
            Node {
                name: "validate_reports",
                func: validate_reports,
                input: (
                    &cat.reports.temperature_chart,
                    &cat.reports.rainfall_chart,
                ),
                output: (&cat.reports.validation_passed,),
            },
        )
    }

    fn hooks() -> impl Hooks {
        (
            LoggingHook::new(),
            VizHook::new("http://localhost:8080".to_string()),
        )
    }
}

// ---------------------------------------------------------------------------
// Fixture generation
// ---------------------------------------------------------------------------

pub fn weather_data_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("examples").join("weather_data")
}

pub fn write_fixtures(dir: &std::path::Path) {
    use std::fs;

    fs::create_dir_all(dir.join("stations")).unwrap();

    // Station North: cool climate
    fs::write(
        dir.join("stations/north.csv"),
        "\
date,temperature,humidity,rainfall,wind_speed
2024-01-15,-5.2,45.0,12.3,22.1
2024-02-15,-3.1,48.0,15.6,19.8
2024-03-15,2.4,52.0,20.1,17.3
2024-04-15,8.7,55.0,25.4,14.2
2024-05-15,14.3,58.0,30.2,12.5
2024-06-15,19.8,62.0,28.7,10.1
2024-07-15,22.1,65.0,18.4,9.3
2024-08-15,20.5,63.0,22.6,10.8
2024-09-15,15.2,58.0,26.3,13.4
2024-10-15,8.1,53.0,23.8,16.7
2024-11-15,1.3,49.0,18.2,20.3
2024-12-15,-4.7,46.0,14.1,23.5
",
    )
    .unwrap();

    // Station South: warm climate
    fs::write(
        dir.join("stations/south.csv"),
        "\
date,temperature,humidity,rainfall,wind_speed
2024-01-15,18.4,70.0,45.2,8.3
2024-02-15,19.1,72.0,52.3,7.9
2024-03-15,22.3,68.0,38.1,9.2
2024-04-15,25.6,65.0,22.7,10.5
2024-05-15,28.9,60.0,15.3,11.8
2024-06-15,32.1,55.0,5.2,13.4
2024-07-15,34.5,52.0,2.1,14.7
2024-08-15,33.8,54.0,4.8,13.9
2024-09-15,30.2,58.0,12.4,11.6
2024-10-15,26.4,63.0,28.5,9.8
2024-11-15,21.7,67.0,40.1,8.5
2024-12-15,19.2,71.0,48.6,8.1
",
    )
    .unwrap();

    // Station East: temperate, with an anomalous July reading (50.0C!)
    fs::write(
        dir.join("stations/east.csv"),
        "\
date,temperature,humidity,rainfall,wind_speed
2024-01-15,3.2,55.0,30.1,15.2
2024-02-15,4.8,53.0,28.4,14.6
2024-03-15,9.1,50.0,25.3,13.1
2024-04-15,14.5,52.0,20.7,11.4
2024-05-15,19.7,55.0,18.2,10.2
2024-06-15,24.3,58.0,12.5,9.1
2024-07-15,50.0,40.0,0.0,25.0
2024-08-15,25.1,57.0,10.3,9.5
2024-09-15,20.4,54.0,16.8,11.3
2024-10-15,14.2,52.0,22.1,13.7
2024-11-15,7.8,54.0,27.5,15.1
2024-12-15,4.1,56.0,31.2,16.3
",
    )
    .unwrap();

    // Station metadata YAML
    fs::write(
        dir.join("station_metadata.yml"),
        "\
north:
  name: Station North
  latitude: 64.1
  longitude: 21.9
  elevation: 450
south:
  name: Station South
  latitude: 28.5
  longitude: -16.3
  elevation: 52
east:
  name: Station East
  latitude: 48.2
  longitude: 16.4
  elevation: 171
",
    )
    .unwrap();

    // Catalog YAML
    fs::write(
        dir.join("catalog.yml"),
        format!(
            "\
sources:
  station_readings:
    path: {d}/stations
    ext: csv
    dataset:
      path: ''
  station_metadata:
    path: {d}/station_metadata.yml
analysis:
  combined_readings:
    path: {d}/combined_readings.csv
  weather_summary: {{}}
  anomalies:
    path: {d}/anomalies.csv
reports:
  temperature_chart:
    path: {d}/temperature_chart.json
  rainfall_chart:
    path: {d}/rainfall_chart.json
  validation_passed: {{}}
",
            d = dir.display()
        ),
    )
    .unwrap();

    // Params YAML
    fs::write(
        dir.join("params.yml"),
        "\
analysis:
  anomaly_threshold: 2.0
  baseline:
    start_month: 1
    end_month: 12
display:
  chart_title: Weather Analysis 2024
  top_n: 5
",
    )
    .unwrap();
}
