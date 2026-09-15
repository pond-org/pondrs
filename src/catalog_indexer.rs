//! Catalog indexer: maps dataset pointer IDs to human-readable field names.
//!
//! Uses a custom serde `Serializer` to introspect catalog structs. Since serde's
//! generated code passes `&self.field` to `serialize_field`, and pipeline nodes
//! store references to the same fields, the pointer addresses match — giving us
//! a `ptr_id -> name` mapping.
//!
//! The indexer stops recursing at Dataset/Param boundaries to avoid
//! first-field address collisions (`ptr_to_id(&s) == ptr_to_id(&s.first_field)`).
//! Detection uses the serde struct name: types ending with `"Dataset"` or
//! named `"Param"` are treated as leaves.

use std::prelude::v1::*;
use std::collections::HashMap;
use std::fmt;

use serde::ser::{self, Serialize};

use crate::naming::{is_leaf_type, type_ident};
use crate::pipeline::{DatasetRef, StepMeta, StepsMeta};
use crate::pipeline::ptr_to_id;

/// One name the walk recorded at a pointer address, with the serde struct name
/// of the value found *at* that address, if it had one.
///
/// A single address can collect several of these: a struct shares its address
/// with its first field, so a container and the dataset inside it are the same
/// pointer. Entries are pushed in increasing depth order, so the last one is
/// the name the index resolves to.
struct IndexEntry {
    name: String,
    serde_ident: Option<&'static str>,
}

/// A mapping from dataset pointer IDs to their human-readable names.
pub struct CatalogIndex {
    names: HashMap<usize, String>,
    /// Every name seen per address, not just the winner — the raw material for
    /// [`check_catalog`].
    entries: HashMap<usize, Vec<IndexEntry>>,
}

impl CatalogIndex {
    /// Look up the name for a dataset pointer ID.
    pub fn get(&self, ptr_id: usize) -> Option<&str> {
        self.names.get(&ptr_id).map(String::as_str)
    }

    /// Return the inner map.
    pub fn into_inner(self) -> HashMap<usize, String> {
        self.names
    }
}

/// Build a `CatalogIndex` from any catalog struct that derives `Serialize`.
///
/// Must be called on the same catalog instance whose fields are referenced
/// by pipeline nodes — pointer addresses must match.
/// Build a `CatalogIndex` from both catalog and params structs.
///
/// Wraps them in a single serializable context so all dataset fields
/// from both are indexed in one pass.
pub fn index_catalog_with_params(catalog: &impl Serialize, params: &impl Serialize) -> CatalogIndex {
    #[derive(serde::Serialize)]
    struct Context<'a, C: Serialize, P: Serialize> {
        catalog: &'a C,
        params: &'a P,
    }
    let context = Context { catalog, params };
    index_catalog(&context)
}

pub fn index_catalog(catalog: &impl Serialize) -> CatalogIndex {
    let mut indexer = CatalogIndexer {
        entries: HashMap::new(),
        prefix: String::new(),
        pending_map_key: None,
        capturing_map_key: false,
        pending_ptr: None,
    };
    catalog.serialize(&mut indexer).ok();

    // The resolved name is the deepest entry at each address — the same
    // last-write-wins rule the walk used before it kept the losers around.
    let names = indexer
        .entries
        .iter()
        .filter_map(|(id, v)| v.last().map(|e| (*id, e.name.clone())))
        .collect();

    CatalogIndex { names, entries: indexer.entries }
}

struct CatalogIndexer {
    entries: HashMap<usize, Vec<IndexEntry>>,
    prefix: String,
    pending_map_key: Option<String>,
    capturing_map_key: bool,
    /// Address whose value is about to be serialized, awaiting its serde struct
    /// name. Set immediately before a `value.serialize(..)` that descends into a
    /// freshly recorded entry, and taken by `serialize_struct` /
    /// `serialize_newtype_struct` on the way in.
    pending_ptr: Option<usize>,
}

impl CatalogIndexer {
    fn full_name(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", self.prefix, key)
        }
    }

    /// Record `name` at `ptr_id` and arm the serde-name stamp for it.
    fn push_entry(&mut self, ptr_id: usize, name: String) {
        self.entries
            .entry(ptr_id)
            .or_default()
            .push(IndexEntry { name, serde_ident: None });
        self.pending_ptr = Some(ptr_id);
    }

    /// Stamp the serde struct name of the value now being serialized onto the
    /// entry that was armed for it.
    fn stamp_serde_ident(&mut self, name: &'static str) {
        if let Some(ptr_id) = self.pending_ptr.take() {
            if let Some(entry) = self.entries.get_mut(&ptr_id).and_then(|v| v.last_mut()) {
                entry.serde_ident = Some(name);
            }
        }
    }
}

// Error type for our no-op serializer.
#[derive(Debug)]
struct IndexerError;

impl fmt::Display for IndexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "catalog indexer error")
    }
}

impl std::error::Error for IndexerError {}

impl ser::Error for IndexerError {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Self
    }
}

/// Non-fatal diagnostic from [`check_catalog`]: a dataset the pipeline uses
/// that the catalog walk could not name, or named wrongly.
///
/// Neither case stops a pipeline from running — both only corrupt the names
/// that reach logs, hooks and viz.
#[non_exhaustive]
#[derive(Debug)]
pub enum CatalogWarning {
    /// The catalog walk never reached this dataset, so it has no name at all.
    UnnamedDataset {
        node_name: &'static str,
        dataset_id: usize,
        type_name: &'static str,
    },
    /// The walk reached the dataset but a *deeper* entry at the same address
    /// won, so the dataset resolves to an interior field name.
    MisresolvedName {
        node_name: &'static str,
        dataset_id: usize,
        type_name: &'static str,
        /// The name the index currently resolves to (the wrong one).
        resolved: String,
        /// The name the dataset itself was recorded under.
        expected: String,
    },
    /// A catalog entry that the naming convention identifies as a dataset or
    /// param, but which no node in the pipeline reads or writes.
    ///
    /// Often dead config — but not always, and this is the one warning here
    /// that can be outright wrong rather than merely incomplete. A param read
    /// while *building* the pipeline (a flag gating whether a node is included)
    /// is never seen by this check: it is a plain field access in the pipeline
    /// function, with nothing to observe. See [`check_catalog`].
    UnusedCatalogEntry { name: String },
}

impl fmt::Display for CatalogWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnnamedDataset { node_name, dataset_id, type_name } => {
                write!(
                    f,
                    "dataset {dataset_id:#x} of type `{type_name}` used by node '{node_name}' \
                     was not found in the catalog, so it has no name in logs or viz; \
                     usual causes: a `#[serde(skip)]` field, a dataset inside a `Vec` or tuple \
                     (the indexer does not descend into sequences), a hand-written `Serialize` \
                     that does not pass `&self.field` through, or a dataset constructed outside \
                     the catalog"
                )
            }
            Self::UnusedCatalogEntry { name } => {
                write!(
                    f,
                    "catalog entry '{name}' is not read or written by any node; \
                     it is dead config unless it is a param the pipeline function reads \
                     while building the pipeline (a flag gating a node, say), or the \
                     catalog is deliberately shared with another pipeline"
                )
            }
            Self::MisresolvedName { node_name, dataset_id, type_name, resolved, expected } => {
                write!(
                    f,
                    "dataset {dataset_id:#x} of type `{type_name}` used by node '{node_name}' \
                     resolves to '{resolved}' instead of '{expected}'; the type ident does not \
                     end in `Dataset`, so the catalog indexer recursed into it and an interior \
                     field name won"
                )
            }
        }
    }
}

/// Cross-check the datasets a pipeline uses against the names the catalog walk
/// resolves for them.
///
/// Complements [`StepsMeta::for_each_warning`], which works from the pipeline
/// alone: this needs the catalog, and so exists only under `std`.
///
/// Every warning is reported; nothing here is fatal and there is no error path.
///
/// # Pass the unfiltered pipeline
///
/// A node filter (`--nodes`, `--from-nodes`, `--to-nodes`) hides datasets. For
/// the naming warnings that only means a hidden dataset goes unchecked — a false
/// negative. For [`UnusedCatalogEntry`] it inverts: every dataset belonging to a
/// filtered-out node looks unused, and the output becomes noise.
///
/// # `UnusedCatalogEntry` has legitimate causes
///
/// It is the one warning here that can be *wrong* rather than merely
/// incomplete. Two shapes it cannot see:
///
/// - **A param read while building the pipeline.** `if params.include_report.0 {
///   steps.push(..) }` is a plain field access in the pipeline function — no
///   node names the param, and there is nothing for this check to observe.
/// - **A catalog shared between several pipelines**, which will always have
///   entries this one does not touch.
///
/// [`UnusedCatalogEntry`]: CatalogWarning::UnusedCatalogEntry
pub fn check_catalog(
    pipe: &impl StepsMeta,
    catalog: &impl Serialize,
    params: &impl Serialize,
    report: &mut dyn FnMut(&CatalogWarning),
) {
    let index = index_catalog_with_params(catalog, params);
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
    pipe.for_each_meta(&mut |item| {
        check_step(item, &index, &mut seen, report);
    });
    report_unused_entries(&index, &seen, report);
}

/// Report catalog entries the convention identifies as datasets that no node
/// touched.
///
/// Only the *winning* entry at each address is considered, and only when its
/// serde struct name says it is a dataset or param. An entry the convention
/// cannot see — a `CellDataset`, whose hand-written `Serialize` emits no struct
/// name, or an unconventionally-named type whose interior field won — is passed
/// over rather than guessed at, the same conservative rule the naming checks use.
fn report_unused_entries(
    index: &CatalogIndex,
    seen: &std::collections::HashSet<usize>,
    report: &mut dyn FnMut(&CatalogWarning),
) {
    let mut unused: Vec<&str> = index
        .entries
        .iter()
        .filter(|(id, _)| !seen.contains(*id))
        .filter_map(|(_, v)| v.last())
        .filter(|e| e.serde_ident.is_some_and(is_leaf_type))
        .map(|e| e.name.as_str())
        .collect();

    // `HashMap` iteration order is nondeterministic; the output must not be.
    unused.sort_unstable();

    for name in unused {
        report(&CatalogWarning::UnusedCatalogEntry { name: name.to_string() });
    }
}

fn check_step(
    item: &dyn StepMeta,
    index: &CatalogIndex,
    seen: &mut std::collections::HashSet<usize>,
    report: &mut dyn FnMut(&CatalogWarning),
) {
    if !item.is_leaf() {
        item.for_each_child(&mut |child| check_step(child, index, seen, report));
        return;
    }

    let node_name = item.name();
    let mut check_ref = |d: &DatasetRef| {
        if !seen.insert(d.id) {
            return;
        }
        let type_name = d.meta.type_string();
        let Some(entries) = index.entries.get(&d.id) else {
            report(&CatalogWarning::UnnamedDataset {
                node_name,
                dataset_id: d.id,
                type_name,
            });
            return;
        };

        // Positive identification only: warn when an entry can be shown to be
        // this very dataset (its serde struct name matches the pipeline's real
        // type ident) and a deeper entry beat it. Anything we cannot identify —
        // a dataset with a hand-written `Serialize` that emits no struct name,
        // say — is left alone rather than guessed at.
        let ident = type_ident(type_name);
        if let Some(i) = entries.iter().rposition(|e| e.serde_ident == Some(ident)) {
            if i + 1 != entries.len() {
                report(&CatalogWarning::MisresolvedName {
                    node_name,
                    dataset_id: d.id,
                    type_name,
                    resolved: entries[entries.len() - 1].name.clone(),
                    expected: entries[i].name.clone(),
                });
            }
        }
    };
    item.for_each_input(&mut check_ref);
    item.for_each_output(&mut check_ref);
}

/// Struct serializer that either recurses into fields or acts as a no-op leaf.
enum StructSerializer<'a> {
    Recurse(&'a mut CatalogIndexer),
    Leaf,
}

// The Serializer implementation. We only care about serialize_struct;
// everything else is a no-op.
impl<'a> ser::Serializer for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = Self;

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        self.stamp_serde_ident(name);
        if is_leaf_type(name) {
            Ok(StructSerializer::Leaf)
        } else {
            Ok(StructSerializer::Recurse(self))
        }
    }

    // All other serializer methods are no-ops.
    fn serialize_bool(self, _v: bool) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_i8(self, _v: i8) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_i16(self, _v: i16) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_i32(self, _v: i32) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_i64(self, _v: i64) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_u8(self, _v: u8) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_u16(self, _v: u16) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_u32(self, _v: u32) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_u64(self, _v: u64) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_f32(self, _v: f32) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_f64(self, _v: f64) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_char(self, _v: char) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_str(self, v: &str) -> Result<(), Self::Error> {
        if self.capturing_map_key {
            self.pending_map_key = Some(v.to_string());
            self.capturing_map_key = false;
        }
        Ok(())
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_none(self) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, _v: &T) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_unit(self) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_unit_variant(self, _name: &'static str, _idx: u32, _variant: &'static str) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, name: &'static str, value: &T) -> Result<(), Self::Error> {
        self.stamp_serde_ident(name);
        if is_leaf_type(name) {
            return Ok(());
        }
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _idx: u32, _variant: &'static str, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> { Ok(self) }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> { Ok(self) }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> { Ok(self) }
    fn serialize_tuple_variant(self, _name: &'static str, _idx: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> { Ok(self) }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> { Ok(self) }
    fn serialize_struct_variant(self, _name: &'static str, _idx: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Self::Error> { Ok(self) }
}

// SerializeStruct — captures field pointers, with leaf detection.
impl ser::SerializeStruct for StructSerializer<'_> {
    type Ok = ();
    type Error = IndexerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        let indexer = match self {
            StructSerializer::Leaf => return Ok(()),
            StructSerializer::Recurse(indexer) => indexer,
        };

        let ptr_id = ptr_to_id(value);
        let name = indexer.full_name(key);

        // Record this field's pointer ID and name, and arm the serde-name stamp.
        indexer.push_entry(ptr_id, name.clone());

        // Recurse into nested structs: temporarily set prefix, serialize, restore.
        let prev_prefix = std::mem::replace(&mut indexer.prefix, name);
        value.serialize(&mut **indexer).ok();
        indexer.prefix = prev_prefix;
        indexer.pending_ptr = None;

        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

// No-op implementations for the other SerializeX traits.
impl ser::SerializeSeq for &mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl ser::SerializeTuple for &mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl ser::SerializeTupleStruct for &mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl ser::SerializeTupleVariant for &mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl ser::SerializeMap for &mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.capturing_map_key = true;
        key.serialize(&mut **self)?;
        // capturing_map_key is cleared by serialize_str once the key is captured.
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        if let Some(key) = self.pending_map_key.take() {
            let ptr_id = ptr_to_id(value);
            let name = self.full_name(&key);
            self.push_entry(ptr_id, name.clone());

            let prev_prefix = std::mem::replace(&mut self.prefix, name);
            value.serialize(&mut **self).ok();
            self.prefix = prev_prefix;
            self.pending_ptr = None;
        }
        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl ser::SerializeStructVariant for &mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _key: &'static str, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{MemoryDataset, Param};
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestCatalog {
        alpha: MemoryDataset<i32>,
        beta: MemoryDataset<i32>,
    }

    #[test]
    fn test_index_flat_catalog() {
        let catalog = TestCatalog {
            alpha: MemoryDataset::new(),
            beta: MemoryDataset::new(),
        };

        let index = index_catalog(&catalog);

        assert_eq!(index.get(ptr_to_id(&catalog.alpha)), Some("alpha"));
        assert_eq!(index.get(ptr_to_id(&catalog.beta)), Some("beta"));
    }

    #[derive(Serialize)]
    struct NestedCatalog {
        inner: TestCatalog,
        gamma: MemoryDataset<i32>,
    }

    #[test]
    fn test_index_nested_catalog() {
        let catalog = NestedCatalog {
            inner: TestCatalog {
                alpha: MemoryDataset::new(),
                beta: MemoryDataset::new(),
            },
            gamma: MemoryDataset::new(),
        };

        let index = index_catalog(&catalog);

        assert_eq!(index.get(ptr_to_id(&catalog.inner.alpha)), Some("inner.alpha"));
        assert_eq!(index.get(ptr_to_id(&catalog.inner.beta)), Some("inner.beta"));
        assert_eq!(index.get(ptr_to_id(&catalog.gamma)), Some("gamma"));
    }

    // A dataset whose first field (path) shares the struct's address.
    // The indexer must stop at the "Dataset" boundary and NOT record "ds.path".
    #[derive(Serialize)]
    struct PathDataset {
        path: String,
    }

    #[derive(Serialize)]
    struct PathCatalog {
        ds: PathDataset,
        other: MemoryDataset<i32>,
    }

    #[test]
    fn test_dataset_first_field_collision() {
        let catalog = PathCatalog {
            ds: PathDataset { path: "data.csv".into() },
            other: MemoryDataset::new(),
        };

        // Verify the collision exists: struct and first field share an address.
        assert_eq!(ptr_to_id(&catalog.ds), ptr_to_id(&catalog.ds.path));

        let index = index_catalog(&catalog);

        // The indexer must choose "ds" (the dataset), not "ds.path" (internal field).
        assert_eq!(index.get(ptr_to_id(&catalog.ds)), Some("ds"));
        assert_eq!(index.get(ptr_to_id(&catalog.other)), Some("other"));
    }

    // Container struct whose first field is a dataset — both share the same address.
    // The indexer must recurse into the container and record the deeper dataset name.
    #[derive(Serialize)]
    struct InnerCatalog {
        first: MemoryDataset<i32>,
        second: MemoryDataset<i32>,
    }

    #[derive(Serialize)]
    struct OuterCatalog {
        inner: InnerCatalog,
    }

    #[test]
    fn test_container_first_field_collision() {
        let catalog = OuterCatalog {
            inner: InnerCatalog {
                first: MemoryDataset::new(),
                second: MemoryDataset::new(),
            },
        };

        // Verify the collision exists: container and its first dataset share an address.
        assert_eq!(ptr_to_id(&catalog.inner), ptr_to_id(&catalog.inner.first));

        let index = index_catalog(&catalog);

        // The deeper name "inner.first" must win over the container name "inner".
        assert_eq!(index.get(ptr_to_id(&catalog.inner.first)), Some("inner.first"));
        assert_eq!(index.get(ptr_to_id(&catalog.inner.second)), Some("inner.second"));
    }

    // Param<T> is a newtype — &param == &param.0.
    // When T is a struct, the indexer must NOT recurse into T's fields.
    #[derive(Serialize, Clone)]
    struct MyConfig {
        value: f64,
    }

    #[derive(Serialize)]
    struct ParamsCatalog {
        cfg: Param<MyConfig>,
        threshold: Param<f64>,
    }

    #[test]
    fn test_param_first_field_collision() {
        let catalog = ParamsCatalog {
            cfg: Param(MyConfig { value: 42.0 }),
            threshold: Param(1.5),
        };

        // Verify the collision: Param and its inner T share the same address.
        assert_eq!(ptr_to_id(&catalog.cfg), ptr_to_id(&catalog.cfg.0));

        let index = index_catalog(&catalog);

        // Must be "cfg", not "cfg.value".
        assert_eq!(index.get(ptr_to_id(&catalog.cfg)), Some("cfg"));
        assert_eq!(index.get(ptr_to_id(&catalog.threshold)), Some("threshold"));
    }

    // --- check_catalog -----------------------------------------------------

    use crate::datasets::Dataset;
    use crate::error::PondError;
    use crate::pipeline::Node;

    /// A dataset whose ident breaks the `*Dataset` convention, with a first
    /// field that shares its address — the `S3Store` shape from the plan.
    #[derive(Serialize)]
    struct StoreThing {
        bucket: String,
    }

    impl Dataset for StoreThing {
        type LoadItem = i32;
        type SaveItem = i32;
        type Error = PondError;
        fn load(&self) -> Result<i32, PondError> {
            Ok(i32::try_from(self.bucket.len()).unwrap_or(i32::MAX))
        }
        fn save(&self, _output: i32) -> Result<(), PondError> {
            Ok(())
        }
    }

    /// Collect the warnings `check_catalog` reports, as formatted strings paired
    /// with a coarse variant tag.
    fn collect(
        pipe: &impl crate::pipeline::StepsMeta,
        catalog: &impl Serialize,
    ) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        check_catalog(pipe, catalog, &(), &mut |w| {
            let tag = match w {
                CatalogWarning::UnnamedDataset { .. } => "unnamed",
                CatalogWarning::MisresolvedName { .. } => "misresolved",
                CatalogWarning::UnusedCatalogEntry { .. } => "unused",
            };
            out.push((tag, w.to_string()));
        });
        out
    }

    #[derive(Serialize)]
    struct StoreCatalog {
        s3: StoreThing,
        other: MemoryDataset<i32>,
    }

    #[test]
    fn check_catalog_flags_misresolved_name() {
        let catalog = StoreCatalog {
            s3: StoreThing { bucket: "b".into() },
            other: MemoryDataset::new(),
        };
        // The premise: the dataset and its first field share an address.
        assert_eq!(ptr_to_id(&catalog.s3), ptr_to_id(&catalog.s3.bucket));

        let pipe = (Node {
            name: "upload",
            func: |v: i32| (v,),
            input: (&catalog.s3,),
            output: (&catalog.other,),
        },);

        let mut found = None;
        check_catalog(&pipe, &catalog, &(), &mut |w| {
            if let CatalogWarning::MisresolvedName { resolved, expected, .. } = w {
                found = Some((resolved.clone(), expected.clone()));
            }
        });
        assert_eq!(
            found,
            Some(("catalog.s3.bucket".to_string(), "catalog.s3".to_string()))
        );
    }

    #[test]
    fn check_catalog_accepts_conventional_catalog() {
        let catalog = TestCatalog {
            alpha: MemoryDataset::new(),
            beta: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.alpha,),
            output: (&catalog.beta,),
        },);
        assert_eq!(collect(&pipe, &catalog), Vec::new());
    }

    // A dataset with a hand-written `Serialize` that emits no struct name at
    // all — the false positive a "was this a leaf boundary?" rule would hit.
    #[derive(Serialize)]
    struct CellCatalog {
        cell: crate::datasets::CellDataset<i32>,
        out: MemoryDataset<i32>,
    }

    #[test]
    fn check_catalog_stays_quiet_for_unidentifiable_datasets() {
        let catalog = CellCatalog {
            cell: crate::datasets::CellDataset::new(),
            out: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.cell,),
            output: (&catalog.out,),
        },);
        assert_eq!(collect(&pipe, &catalog), Vec::new());
    }

    #[test]
    fn check_catalog_accepts_container_whose_first_field_is_a_dataset() {
        let catalog = OuterCatalog {
            inner: InnerCatalog {
                first: MemoryDataset::new(),
                second: MemoryDataset::new(),
            },
        };
        assert_eq!(ptr_to_id(&catalog.inner), ptr_to_id(&catalog.inner.first));

        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.inner.first,),
            output: (&catalog.inner.second,),
        },);
        assert_eq!(collect(&pipe, &catalog), Vec::new());
    }

    #[derive(Serialize)]
    struct SkipCatalog {
        visible: MemoryDataset<i32>,
        #[serde(skip)]
        hidden: MemoryDataset<i32>,
    }

    #[test]
    fn check_catalog_flags_serde_skipped_dataset() {
        let catalog = SkipCatalog {
            visible: MemoryDataset::new(),
            hidden: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.visible,),
            output: (&catalog.hidden,),
        },);
        let warnings = collect(&pipe, &catalog);
        assert_eq!(warnings.len(), 1, "got {warnings:?}");
        assert_eq!(warnings[0].0, "unnamed");
    }

    #[derive(Serialize)]
    struct VecCatalog {
        items: Vec<MemoryDataset<i32>>,
        out: MemoryDataset<i32>,
    }

    #[test]
    fn check_catalog_flags_dataset_inside_a_vec() {
        // The indexer's `SerializeSeq` element methods are no-ops, so nothing
        // inside a `Vec` is ever named.
        let catalog = VecCatalog {
            items: vec![MemoryDataset::new()],
            out: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.items[0],),
            output: (&catalog.out,),
        },);
        let warnings = collect(&pipe, &catalog);
        assert_eq!(warnings.len(), 1, "got {warnings:?}");
        assert_eq!(warnings[0].0, "unnamed");
    }

    // --- UnusedCatalogEntry ------------------------------------------------

    /// The names reported as unused, in the order `check_catalog` emits them.
    fn unused_names(
        pipe: &impl crate::pipeline::StepsMeta,
        catalog: &impl Serialize,
    ) -> Vec<String> {
        let mut out = Vec::new();
        check_catalog(pipe, catalog, &(), &mut |w| {
            if let CatalogWarning::UnusedCatalogEntry { name } = w {
                out.push(name.clone());
            }
        });
        out
    }

    #[derive(Serialize)]
    struct WideCatalog {
        used_in: MemoryDataset<i32>,
        used_out: MemoryDataset<i32>,
        zulu: MemoryDataset<i32>,
        alpha: MemoryDataset<i32>,
    }

    #[test]
    fn check_catalog_flags_unused_entries_in_sorted_order() {
        let catalog = WideCatalog {
            used_in: MemoryDataset::new(),
            used_out: MemoryDataset::new(),
            zulu: MemoryDataset::new(),
            alpha: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.used_in,),
            output: (&catalog.used_out,),
        },);

        // Sorted, so the output does not depend on HashMap iteration order.
        assert_eq!(
            unused_names(&pipe, &catalog),
            vec!["catalog.alpha".to_string(), "catalog.zulu".to_string()]
        );
    }

    #[test]
    fn check_catalog_does_not_flag_used_entries() {
        let catalog = TestCatalog {
            alpha: MemoryDataset::new(),
            beta: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32| (v,),
            input: (&catalog.alpha,),
            output: (&catalog.beta,),
        },);
        assert_eq!(unused_names(&pipe, &catalog), Vec::<String>::new());
    }

    #[test]
    fn check_catalog_flags_unused_params() {
        let catalog = TestCatalog {
            alpha: MemoryDataset::new(),
            beta: MemoryDataset::new(),
        };
        let params = ParamsCatalog {
            cfg: Param(MyConfig { value: 1.0 }),
            threshold: Param(1.5),
        };
        let pipe = (Node {
            name: "n1",
            func: |v: i32, _t: f64| (v,),
            input: (&catalog.alpha, &params.threshold),
            output: (&catalog.beta,),
        },);

        let mut out = Vec::new();
        check_catalog(&pipe, &catalog, &params, &mut |w| {
            if let CatalogWarning::UnusedCatalogEntry { name } = w {
                out.push(name.clone());
            }
        });
        // `threshold` is read by the node; `cfg` is dead config.
        assert_eq!(out, vec!["params.cfg".to_string()]);
    }

    #[test]
    fn check_catalog_does_not_flag_entries_the_convention_cannot_see() {
        // `CellDataset` serializes as a unit, so the walk never learns a struct
        // name for it and cannot claim it is a dataset. Unused or not, it is
        // passed over rather than guessed at.
        let catalog = CellCatalog {
            cell: crate::datasets::CellDataset::new(),
            out: MemoryDataset::new(),
        };
        let pipe = (Node {
            name: "n1",
            func: |_v: i32| (),
            input: (&catalog.out,),
            output: (),
        },);
        assert_eq!(unused_names(&pipe, &catalog), Vec::<String>::new());
    }

    #[test]
    fn check_catalog_does_not_flag_container_structs() {
        // Only leaf entries count: `inner` itself must not be reported on top of
        // the datasets inside it.
        let catalog = OuterCatalog {
            inner: InnerCatalog {
                first: MemoryDataset::new(),
                second: MemoryDataset::new(),
            },
        };
        let pipe = (Node {
            name: "n1",
            func: |_v: i32| (),
            input: (&catalog.inner.first,),
            output: (),
        },);
        assert_eq!(
            unused_names(&pipe, &catalog),
            vec!["catalog.inner.second".to_string()]
        );
    }
}
