//! Parameter dataset - read-only values.

use core::convert::Infallible;

use serde::{Deserialize, Serialize};

use super::Dataset;

#[derive(Serialize, Deserialize)]
pub struct Param<T: Clone>(pub T);

impl<T: Clone> Dataset for Param<T> {
    type LoadItem = T;
    type SaveItem = ();
    type Error = Infallible;

    fn load(&self) -> Result<Self::LoadItem, Infallible> {
        Ok(self.0.clone())
    }

    fn save(&self, _output: Self::SaveItem) -> Result<(), Infallible> {
        Ok(())
    }

    fn is_param(&self) -> bool { true }
}
