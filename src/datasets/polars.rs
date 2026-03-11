//! Polars DataFrame dataset.

use std::prelude::v1::*;
use polars::prelude::{CsvParseOptions, CsvReadOptions, CsvWriter, DataFrame, ParquetReader, ParquetWriter, SerReader, SerWriter};
use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::{Dataset, FileDataset};

const MAX_ROWS: usize = 50;

fn dataframe_to_html(df: &DataFrame) -> String {
    let cols = df.columns();
    let n = df.height().min(MAX_ROWS);

    let mut s = String::from(
        "<table style=\"border-collapse:collapse;font-size:13px;font-family:monospace\">\n\
         <thead><tr>",
    );
    for col in cols {
        s.push_str(&format!(
            "<th style=\"border:1px solid #ccc;padding:4px 8px;background:#f5f5f5;text-align:left\">{}</th>",
            col.name()
        ));
    }
    s.push_str("</tr></thead>\n<tbody>\n");

    for i in 0..n {
        s.push_str("<tr>");
        for col in cols {
            let val = match col.get(i) {
                Ok(v) => format!("{v}"),
                Err(_) => String::new(),
            };
            s.push_str(&format!(
                "<td style=\"border:1px solid #ccc;padding:4px 8px\">{val}</td>"
            ));
        }
        s.push_str("</tr>\n");
    }

    s.push_str("</tbody></table>");
    if df.height() > MAX_ROWS {
        s.push_str(&format!(
            "<p style=\"color:#888;font-size:12px\">Showing {MAX_ROWS} of {} rows</p>",
            df.height()
        ));
    }
    s
}

fn default_separator() -> char { ',' }
fn default_has_header() -> bool { true }

/// CSV file dataset backed by Polars. Supports configurable separator,
/// header, and row skipping.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolarsCsvDataset {
    pub path: String,
    #[serde(default = "default_separator")]
    pub separator: char,
    #[serde(default = "default_has_header")]
    pub has_header: bool,
    #[serde(default)]
    pub skip_rows: usize,
}

impl PolarsCsvDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            separator: default_separator(),
            has_header: default_has_header(),
            skip_rows: 0,
        }
    }
}

impl Dataset for PolarsCsvDataset {
    type LoadItem = DataFrame;
    type SaveItem = DataFrame;
    type Error = PondError;

    fn save(&self, mut df: Self::SaveItem) -> Result<(), PondError> {
        let mut file = std::fs::File::create(&self.path)?;
        CsvWriter::new(&mut file)
            .with_separator(self.separator as u8)
            .finish(&mut df)?;
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let df = CsvReadOptions::default()
            .with_has_header(self.has_header)
            .with_skip_rows(self.skip_rows)
            .with_parse_options(
                CsvParseOptions::default().with_separator(self.separator as u8)
            )
            .try_into_reader_with_file_path(Some(self.path.clone().into()))?
            .finish()?;
        Ok(df)
    }

    fn html(&self) -> Option<String> {
        self.load().ok().map(|df| dataframe_to_html(&df))
    }
}

impl FileDataset for PolarsCsvDataset {
    fn path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}

/// Parquet file dataset backed by Polars.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolarsParquetDataset {
    pub path: String,
}

impl PolarsParquetDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Dataset for PolarsParquetDataset {
    type LoadItem = DataFrame;
    type SaveItem = DataFrame;
    type Error = PondError;

    fn save(&self, mut df: Self::SaveItem) -> Result<(), PondError> {
        let mut file = std::fs::File::create(&self.path)?;
        ParquetWriter::new(&mut file).finish(&mut df)?;
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let file = std::fs::File::open(&self.path)?;
        let df = ParquetReader::new(file).finish()?;
        Ok(df)
    }

    fn html(&self) -> Option<String> {
        self.load().ok().map(|df| dataframe_to_html(&df))
    }
}

impl FileDataset for PolarsParquetDataset {
    fn path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}
