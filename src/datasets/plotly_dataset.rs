//! Plotly chart dataset.

use std::prelude::v1::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PondError;
use super::{Dataset, FileDataset};

/// Dataset that saves a Plotly chart as both `.json` and `.html`, and loads from `.json`.
///
/// The `path` field stores the `.json` file path (e.g., `"output/chart.json"`).
/// On save, a matching `.html` file is written alongside the JSON.
#[derive(Serialize, Deserialize, Clone)]
pub struct PlotlyDataset {
    pub path: String,
}

impl PlotlyDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    fn html_path(&self) -> String {
        if let Some(stem) = self.path.strip_suffix(".json") {
            format!("{stem}.html")
        } else {
            format!("{}.html", self.path)
        }
    }
}

impl Dataset for PlotlyDataset {
    type LoadItem = Value;
    type SaveItem = ::plotly::Plot;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let content = std::fs::read_to_string(&self.path)?;
        let value = serde_json::from_str(&content)?;
        Ok(value)
    }

    fn save(&self, plot: Self::SaveItem) -> Result<(), PondError> {
        std::fs::write(&self.path, plot.to_json())?;
        std::fs::write(self.html_path(), plot.to_html())?;
        Ok(())
    }
}

impl FileDataset for PlotlyDataset {
    fn get_path(&self) -> &str {
        &self.path
    }

    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}
