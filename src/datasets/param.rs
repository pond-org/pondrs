//! Parameter dataset - read-only values.

use serde::{Deserialize, Serialize};

use super::Dataset;

#[derive(Serialize, Deserialize)]
pub struct Param<T: Clone>(pub T);

impl<T: Clone> Dataset for Param<T> {
    type LoadItem = T;
    type SaveItem = ();

    fn load(&self) -> Option<Self::LoadItem> {
        Some(self.0.clone())
    }

    fn save(&self, _output: Self::SaveItem) {}

    fn is_param(&self) -> bool { true }
}
