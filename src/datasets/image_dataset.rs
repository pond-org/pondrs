//! Image dataset backed by the `image` crate.

use std::prelude::v1::*;

use base64::Engine as _;
use image::DynamicImage;
use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::{Dataset, FileDataset};

/// Dataset that loads and saves images using the `image` crate.
///
/// The `path` field stores the image file path (e.g., `"output/photo.png"`).
/// The image format is inferred from the file extension on both load and save.
#[derive(Serialize, Deserialize, Clone)]
pub struct ImageDataset {
    pub path: String,
}

impl ImageDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    fn mime_type(&self) -> &'static str {
        match self.path.rsplit('.').next().unwrap_or("").to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",
            "tiff" | "tif" => "image/tiff",
            _ => "image/png",
        }
    }
}

impl Dataset for ImageDataset {
    type LoadItem = DynamicImage;
    type SaveItem = DynamicImage;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        Ok(image::open(&self.path)?)
    }

    fn save(&self, img: Self::SaveItem) -> Result<(), PondError> {
        img.save(&self.path)?;
        Ok(())
    }

    fn html(&self) -> Option<String> {
        let bytes = std::fs::read(&self.path).ok()?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mime = self.mime_type();
        Some(format!(
            "<img src=\"data:{mime};base64,{encoded}\" style=\"max-width:100%;height:auto\" />"
        ))
    }
}

impl FileDataset for ImageDataset {
    fn path(&self) -> &str {
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
        let path = dir.path().join("img.png");
        let ds = ImageDataset::new(path.to_str().unwrap());
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    #[test]
    fn roundtrip_png() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.png");
        let ds = ImageDataset::new(path.to_str().unwrap());

        let img = DynamicImage::new_rgb8(4, 4);
        ds.save(img).unwrap();

        let loaded = ds.load().unwrap();
        assert_eq!(loaded.width(), 4);
        assert_eq!(loaded.height(), 4);
    }

    #[test]
    fn html_is_some_after_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.png");
        let ds = ImageDataset::new(path.to_str().unwrap());

        let img = DynamicImage::new_rgb8(2, 2);
        ds.save(img).unwrap();

        let meta: &dyn DatasetMeta = &ds;
        let html = meta.html().unwrap();
        assert!(html.contains("data:image/png;base64,"));
        assert!(html.contains("<img"));
    }
}
