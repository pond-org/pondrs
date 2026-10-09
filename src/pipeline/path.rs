//! Qualified step paths: a step's name prefixed by its enclosing groups' names.
//!
//! A path is the `/`-joined names from the top-level step down, e.g.
//! `train/3/fwd`. `check` rejects `/` inside a name and requires sibling names
//! to be unique, which together make every path in a pipeline unique — so a
//! path, unlike a bare name, identifies one step.
//!
//! Runners hand hooks a [`Qualified`] step, whose `name()` is the path; the
//! graph records each node's path, and CLI node filters match against it.

use std::prelude::v1::*;

use super::traits::{DatasetRef, StepMeta};

/// Separator between the segments of a step path.
pub(crate) const SEP: char = '/';

/// Join a parent path and a step's local name.
pub(crate) fn join(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(p) => format!("{p}{SEP}{name}"),
        None => name.to_string(),
    }
}

/// Whether `query` names `path`: equal to it, or a suffix of it starting at a
/// segment boundary (`north/compute` matches `stores/north/compute`, but
/// `orth/compute` does not).
pub(crate) fn matches(path: &str, query: &str) -> bool {
    path.strip_suffix(query)
        .is_some_and(|rest| rest.is_empty() || rest.ends_with(SEP))
}

/// A step whose `name()` is its full path; everything else delegates.
///
/// What runners pass to hooks under `std`, so that same-named nodes in
/// different groups reach hooks under distinct names.
pub(crate) struct Qualified<'a> {
    step: &'a dyn StepMeta,
    path: String,
}

impl<'a> Qualified<'a> {
    pub(crate) fn new(step: &'a dyn StepMeta, path: String) -> Self {
        Self { step, path }
    }
}

impl StepMeta for Qualified<'_> {
    fn name(&self) -> &str {
        &self.path
    }

    fn is_leaf(&self) -> bool {
        self.step.is_leaf()
    }

    fn type_string(&self) -> &'static str {
        self.step.type_string()
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta)) {
        self.step.for_each_child(f);
    }

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.step.for_each_input(f);
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.step.for_each_output(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_paths() {
        assert_eq!(join(None, "a"), "a");
        assert_eq!(join(Some("a/b"), "c"), "a/b/c");
    }

    #[test]
    fn matches_on_segment_boundaries() {
        assert!(matches("stores/north/compute", "stores/north/compute"));
        assert!(matches("stores/north/compute", "north/compute"));
        assert!(matches("stores/north/compute", "compute"));
        assert!(!matches("stores/north/compute", "orth/compute"));
        assert!(!matches("stores/north/compute", "north"));
        assert!(!matches("compute", "stores/compute"));
    }
}
