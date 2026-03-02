#![feature(unboxed_closures, fn_traits, tuple_trait)]

use std::fs;

use polars::prelude::*;
use serde::Serialize;
use tempfile::TempDir;
use yaml_rust2::Yaml;

use pondrs::datasets::{MemoryDataset, Param, PolarsCsvDataset, YamlDataset};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::{Dataset, Node, Pipeline, Runner, SequentialRunner, StepInfo};

// ---------------------------------------------------------------------------
// Test 1: CSV pipeline
//   Reads a CSV, scales a column by a param, writes output CSV,
//   then computes the mean into a MemoryDataset.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CsvCatalog {
    raw_scores: PolarsCsvDataset,
    scaled_scores: PolarsCsvDataset,
    summary: MemoryDataset<f64>,
}

#[derive(Serialize)]
struct CsvParams {
    scale_factor: Param<f64>,
}

#[test]
fn test_csv_pipeline_with_params() {
    let dir = TempDir::new().unwrap();

    let input_path = dir.path().join("raw_scores.csv");
    fs::write(&input_path, "name,score\nalice,80\nbob,90\ncharlie,70\n").unwrap();

    let output_path = dir.path().join("scaled_scores.csv");

    let params = CsvParams {
        scale_factor: Param(1.5),
    };

    let catalog = CsvCatalog {
        raw_scores: PolarsCsvDataset::new(input_path.to_str().unwrap()),
        scaled_scores: PolarsCsvDataset::new(output_path.to_str().unwrap()),
        summary: MemoryDataset::new(),
    };

    let pipe = (
        Node {
            name: "load_and_scale",
            func: |df: DataFrame, scale: f64| -> Result<(DataFrame,), PolarsError> {
                let scores = df.column("score")?.i64()?;
                let scaled: Float64Chunked = scores
                    .into_iter()
                    .map(|v| v.map(|x| x as f64 * scale))
                    .collect_ca("score".into());
                let mut result = df.clone();
                result.replace("score", scaled.into_series().into())?;
                Ok((result,))
            },
            input: (&catalog.raw_scores, &params.scale_factor),
            output: (&catalog.scaled_scores,),
        },
        Node {
            name: "compute_mean",
            func: |df: DataFrame| {
                let mean = df.column("score").unwrap().f64().unwrap().mean().unwrap();
                (mean,)
            },
            input: (&catalog.scaled_scores,),
            output: (&catalog.summary,),
        },
    );

    assert!(pipe.check().is_ok());

    let hooks = (LoggingHook::new(),);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();

    // original scores: 80, 90, 70 → scaled: 120, 135, 105 → mean = 120
    let mean = catalog.summary.load().unwrap();
    assert!((mean - 120.0).abs() < 1e-9);

    // Verify output CSV was written and can be re-read
    let output_df = catalog.scaled_scores.load().unwrap();
    assert_eq!(output_df.height(), 3);
    let scores: Vec<f64> = output_df
        .column("score")
        .unwrap()
        .f64()
        .unwrap()
        .into_no_null_iter()
        .collect();
    assert_eq!(scores, vec![120.0, 135.0, 105.0]);
}

// ---------------------------------------------------------------------------
// Test 2: YAML pipeline
//   Reads a YAML config, extracts a value, combines with params,
//   writes a transformed YAML output.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct YamlCatalog {
    config: YamlDataset,
    result: YamlDataset,
    threshold: MemoryDataset<i64>,
}

#[derive(Serialize)]
struct YamlParams {
    multiplier: Param<i64>,
}

#[test]
fn test_yaml_pipeline_with_params() {
    let dir = TempDir::new().unwrap();

    let input_path = dir.path().join("config.yaml");
    fs::write(&input_path, "settings:\n  threshold: 42\n  enabled: true\n").unwrap();

    let output_path = dir.path().join("result.yaml");

    let params = YamlParams {
        multiplier: Param(3),
    };

    let catalog = YamlCatalog {
        config: YamlDataset::new(input_path.to_str().unwrap()),
        result: YamlDataset::new(output_path.to_str().unwrap()),
        threshold: MemoryDataset::new(),
    };

    let pipe = (
        Node {
            name: "extract_threshold",
            func: |yaml: Yaml| {
                let threshold = yaml["settings"]["threshold"].as_i64().unwrap();
                (threshold,)
            },
            input: (&catalog.config,),
            output: (&catalog.threshold,),
        },
        Pipeline {
            name: "transform",
            steps: (Node {
                name: "build_output",
                func: |value: i64, multiplier: i64| {
                    let scaled = value * multiplier;
                    let mut map = yaml_rust2::yaml::Hash::new();
                    map.insert(Yaml::String("original".into()), Yaml::Integer(value));
                    map.insert(Yaml::String("scaled".into()), Yaml::Integer(scaled));
                    (Yaml::Hash(map),)
                },
                input: (&catalog.threshold, &params.multiplier),
                output: (&catalog.result,),
            },),
            input: (&catalog.threshold, &params.multiplier),
            output: (&catalog.result,),
        },
    );

    assert!(pipe.check().is_ok());

    let hooks = (LoggingHook::new(),);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();

    // Verify intermediate
    assert_eq!(catalog.threshold.load().unwrap(), 42);

    // Verify output YAML
    let output_yaml = catalog.result.load().unwrap();
    assert_eq!(output_yaml["original"].as_i64().unwrap(), 42);
    assert_eq!(output_yaml["scaled"].as_i64().unwrap(), 126); // 42 * 3

    // Verify the file exists on disk
    let raw = fs::read_to_string(&output_path).unwrap();
    assert!(raw.contains("126"));
}

// ---------------------------------------------------------------------------
// Test 3: Mixed CSV + YAML pipeline
//   Reads a threshold from YAML, reads data from CSV, filters using
//   the threshold adjusted by a param offset, writes filtered CSV.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct MixedCatalog {
    config: YamlDataset,
    input_data: PolarsCsvDataset,
    threshold: MemoryDataset<i64>,
    filtered_data: PolarsCsvDataset,
    row_count: MemoryDataset<u32>,
}

#[derive(Serialize)]
struct MixedParams {
    threshold_offset: Param<i64>,
}

#[test]
fn test_mixed_csv_yaml_pipeline() {
    let dir = TempDir::new().unwrap();

    let config_path = dir.path().join("thresholds.yaml");
    fs::write(&config_path, "min_score: 75\n").unwrap();

    let csv_path = dir.path().join("students.csv");
    fs::write(
        &csv_path,
        "student,score\nalice,90\nbob,60\ncharlie,80\ndiana,50\neve,95\n",
    )
    .unwrap();

    let output_path = dir.path().join("passing.csv");

    let params = MixedParams {
        threshold_offset: Param(-5), // effective threshold = 75 + (-5) = 70
    };

    let catalog = MixedCatalog {
        config: YamlDataset::new(config_path.to_str().unwrap()),
        input_data: PolarsCsvDataset::new(csv_path.to_str().unwrap()),
        threshold: MemoryDataset::new(),
        filtered_data: PolarsCsvDataset::new(output_path.to_str().unwrap()),
        row_count: MemoryDataset::new(),
    };

    let pipe = (
        Node {
            name: "read_threshold",
            func: |yaml: Yaml, offset: i64| {
                let base = yaml["min_score"].as_i64().unwrap();
                (base + offset,)
            },
            input: (&catalog.config, &params.threshold_offset),
            output: (&catalog.threshold,),
        },
        Pipeline {
            name: "filter_and_count",
            steps: (
                Node {
                    name: "filter_scores",
                    func: |df: DataFrame, threshold: i64| -> Result<(DataFrame,), PolarsError> {
                        let mask = df.column("score")?.i64()?.gt_eq(threshold);
                        Ok((df.filter(&mask)?,))
                    },
                    input: (&catalog.input_data, &catalog.threshold),
                    output: (&catalog.filtered_data,),
                },
                Node {
                    name: "count_rows",
                    func: |df: DataFrame| (df.height() as u32,),
                    input: (&catalog.filtered_data,),
                    output: (&catalog.row_count,),
                },
            ),
            input: (&catalog.input_data, &catalog.threshold),
            output: (&catalog.filtered_data, &catalog.row_count),
        },
    );

    assert!(pipe.check().is_ok());

    let hooks = (LoggingHook::new(),);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();

    // threshold = 70, passing: alice(90), charlie(80), eve(95)
    assert_eq!(catalog.row_count.load().unwrap(), 3);

    let output_df = catalog.filtered_data.load().unwrap();
    assert_eq!(output_df.height(), 3);

    let names: Vec<&str> = output_df
        .column("student")
        .unwrap()
        .str()
        .unwrap()
        .into_no_null_iter()
        .collect();
    assert!(names.contains(&"alice"));
    assert!(names.contains(&"charlie"));
    assert!(names.contains(&"eve"));
    assert!(!names.contains(&"bob"));
    assert!(!names.contains(&"diana"));
}
