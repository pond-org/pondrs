//! Polars DataFrame dataset.

use std::prelude::v1::*;
use polars::prelude::{CsvReadOptions, CsvWriter, DataFrame, ParquetReader, ParquetWriter, SerReader, SerWriter};
use serde::{Deserialize, Serialize};

use super::{Dataset, FileDataset};

#[derive(Serialize, Deserialize, Clone)]
pub struct PolarsCsvDataset {
    pub path: String,
}

impl PolarsCsvDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Dataset for PolarsCsvDataset {
    type LoadItem = DataFrame;
    type SaveItem = DataFrame;

    fn save(&self, mut df: Self::SaveItem) {
        let mut file = std::fs::File::create(&self.path).unwrap();
        CsvWriter::new(&mut file).finish(&mut df).unwrap();
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some(self.path.clone().into()))
            .unwrap()
            .finish()
            .unwrap();
        Some(df)
    }
}

impl FileDataset for PolarsCsvDataset {
    fn get_path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}

#[derive(Serialize, Deserialize, Clone)]
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

    fn save(&self, mut df: Self::SaveItem) {
        let mut file = std::fs::File::create(&self.path).unwrap();
        ParquetWriter::new(&mut file).finish(&mut df).unwrap();
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let file = std::fs::File::open(&self.path).unwrap();
        let df = ParquetReader::new(file).finish().unwrap();
        Some(df)
    }
}

impl FileDataset for PolarsParquetDataset {
    fn get_path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}
