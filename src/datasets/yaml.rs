//! YAML file dataset.

use std::prelude::v1::*;
use serde::{Deserialize, Serialize};
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use crate::error::PondError;
use super::{Dataset, FileDataset};

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// YAML file dataset. Loads the first document as `yaml_rust2::Yaml`.
#[derive(Serialize, Deserialize, Clone)]
pub struct YamlDataset {
    path: String,
}

impl YamlDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Dataset for YamlDataset {
    type LoadItem = Yaml;
    type SaveItem = Yaml;
    type Error = PondError;

    fn save(&self, yaml: Self::SaveItem) -> Result<(), PondError> {
        let mut out_str = String::new();
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&yaml)?;
        std::fs::write(&self.path, &out_str)?;
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let contents = std::fs::read_to_string(&self.path)?;
        let docs = YamlLoader::load_from_str(&contents)?;
        Ok(docs[0].clone())
    }

    fn html(&self) -> Option<String> {
        let contents = std::fs::read_to_string(&self.path).ok()?;
        Some(format!(
            "<pre style=\"font-family:monospace;font-size:13px;padding:8px;margin:0;overflow:auto\">{}</pre>",
            html_escape(&contents)
        ))
    }
}

impl FileDataset for YamlDataset {
    fn path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}
