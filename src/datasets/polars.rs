//! Polars DataFrame dataset.

use std::prelude::v1::*;
use polars::prelude::{CsvReadOptions, CsvWriter, DataFrame, ParquetReader, ParquetWriter, SerReader, SerWriter};
use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::{Dataset, FileDataset};

#[derive(Serialize, Deserialize, Clone)]
pub struct PolarsCsvDataset {
    #[serde(skip_serializing)]
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
    type Error = PondError;

    fn save(&self, mut df: Self::SaveItem) -> Result<(), PondError> {
        let mut file = std::fs::File::create(&self.path)?;
        CsvWriter::new(&mut file).finish(&mut df)?;
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some(self.path.clone().into()))?
            .finish()?;
        Ok(df)
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
    #[serde(skip_serializing)]
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
}

impl FileDataset for PolarsParquetDataset {
    fn get_path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}
