//! Parameter dataset - read-only values.

#[cfg(feature = "std")]
use std::prelude::v1::*;

use serde::{Deserialize, Serialize};

use crate::error::PondError;

use super::{Dataset, Never};

/// A read-only parameter dataset. Always loads successfully; writing is forbidden.
///
/// `SaveItem` is the uninhabited [`Never`] type, so a node whose output tuple
/// contains a `&Param<T>` cannot type-check: its function would have to produce
/// a value that does not exist.
#[derive(Debug, Serialize, Deserialize)]
pub struct Param<T: Clone>(pub T);

impl<T: Clone + Serialize + 'static> Dataset for Param<T> {
    type LoadItem = T;
    type SaveItem = Never;
    /// `PondError`, not `Infallible`, even though loading a param cannot fail.
    ///
    /// A node's input tuple requires `E: From<D::Error>` of every slot, and a
    /// `Param` appears in nearly every pipeline — `Infallible` here would make
    /// every user error type owe an `From<Infallible>` impl. Blanketing around
    /// that is impossible: any `impl<E> ... for E` overlaps the `E: From<X>`
    /// blanket and coherence rejects it.
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        Ok(self.0.clone())
    }

    /// Param is read-only. `SaveItem` is uninhabited, so this argument cannot
    /// exist and the match discharges it without any runtime code.
    fn save(&self, output: Self::SaveItem) -> Result<(), PondError> {
        match output {}
    }

    fn is_param(&self) -> bool { true }
    fn is_persistent(&self) -> bool { true }

    #[cfg(feature = "std")]
    fn content_hash(&self) -> Option<u64> {
        use core::hash::{Hash, Hasher};

        let yaml = serde_yaml::to_string(&self.0).ok()?;
        let mut hasher = std::hash::DefaultHasher::new();
        yaml.hash(&mut hasher);
        Some(hasher.finish())
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        let yaml = serde_yaml::to_string(&self.0).ok()?;
        let escaped = super::html_escape(&yaml);
        Some(format!(
            "<pre style=\"font-family:monospace;font-size:13px;background:#f5f5f5;\
             border:1px solid #ccc;padding:8px;overflow:auto\">{escaped}</pre>"
        ))
    }
}
