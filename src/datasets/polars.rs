//! Polars DataFrame dataset.

use polars::prelude::{CsvReadOptions, CsvWriter, DataFrame, SerReader, SerWriter};
use serde::{Deserialize, Serialize};

use super::Dataset;

#[derive(Serialize, Deserialize)]
pub struct PolarsDataset {
    path: String,
}

impl PolarsDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Dataset for PolarsDataset {
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
