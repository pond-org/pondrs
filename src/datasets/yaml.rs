//! YAML file dataset.

use serde::{Deserialize, Serialize};
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use super::{Dataset, FileDataset};

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

    fn save(&self, yaml: Self::SaveItem) {
        let mut out_str = String::new();
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&yaml).unwrap();
        std::fs::write(&self.path, &out_str).unwrap();
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let contents = std::fs::read_to_string(&self.path).unwrap();
        let docs = YamlLoader::load_from_str(&contents).unwrap();
        Some(docs[0].clone())
    }
}

impl FileDataset for YamlDataset {
    fn get_path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}
