//! Parameter dataset - read-only values.

#[cfg(feature = "std")]
use std::prelude::v1::*;
use core::convert::Infallible;

use serde::{Deserialize, Serialize};

use super::Dataset;

/// A read-only parameter dataset. Always loads successfully; writing is forbidden.
///
/// The pipeline validator rejects any node that writes to a `Param`.
#[derive(Serialize, Deserialize)]
pub struct Param<T: Clone>(pub T);

impl<T: Clone + Serialize> Dataset for Param<T> {
    type LoadItem = T;
    type SaveItem = ();
    type Error = Infallible;

    fn load(&self) -> Result<Self::LoadItem, Infallible> {
        Ok(self.0.clone())
    }

    /// Param is read-only — the validator prevents writing to params,
    /// so `save()` should never be reached.
    fn save(&self, _output: Self::SaveItem) -> Result<(), Infallible> {
        unreachable!("Param is read-only — save() should never be called")
    }

    fn is_param(&self) -> bool { true }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        let yaml = serde_yaml::to_string(&self.0).ok()?;
        Some(format!(
            "<pre style=\"font-family:monospace;font-size:13px;background:#f5f5f5;\
             border:1px solid #ccc;padding:8px;overflow:auto\">{yaml}</pre>"
        ))
    }
}
