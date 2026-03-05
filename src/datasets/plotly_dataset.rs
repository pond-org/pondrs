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

    fn html(&self) -> Option<String> {
        std::fs::read_to_string(self.html_path()).ok()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::DatasetMeta;
    use tempfile::tempdir;

    #[test]
    fn html_is_none_before_save() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("chart.json");
        let ds = PlotlyDataset::new(json_path.to_str().unwrap());
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    #[test]
    fn html_is_some_after_save() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("chart.json");
        let ds = PlotlyDataset::new(json_path.to_str().unwrap());

        let plot = ::plotly::Plot::new();
        ds.save(plot).unwrap();

        let meta: &dyn DatasetMeta = &ds;
        let html = meta.html();
        assert!(html.is_some());
        let content = html.unwrap();
        assert!(content.contains("<html") || content.contains("<!DOCTYPE"));
    }
}
