//! Catalog indexer: maps dataset pointer IDs to human-readable field names.
//!
//! Uses a custom serde `Serializer` to introspect catalog structs. Since serde's
//! generated code passes `&self.field` to `serialize_field`, and pipeline nodes
//! store references to the same fields, the pointer addresses match — giving us
//! a `ptr_id -> name` mapping.

use std::collections::HashMap;
use std::fmt;

use serde::ser::{self, Serialize};

use crate::core::ptr_to_id;

/// A mapping from dataset pointer IDs to their human-readable names.
pub struct CatalogIndex {
    names: HashMap<usize, String>,
}

impl CatalogIndex {
    /// Look up the name for a dataset pointer ID.
    pub fn get(&self, ptr_id: usize) -> Option<&str> {
        self.names.get(&ptr_id).map(|s| s.as_str())
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
pub fn index_catalog(catalog: &impl Serialize) -> CatalogIndex {
    let mut indexer = CatalogIndexer {
        names: HashMap::new(),
        prefix: String::new(),
    };
    catalog.serialize(&mut indexer).ok();
    CatalogIndex { names: indexer.names }
}

struct CatalogIndexer {
    names: HashMap<usize, String>,
    prefix: String,
}

impl CatalogIndexer {
    fn full_name(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", self.prefix, key)
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
        IndexerError
    }
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
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
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
    fn serialize_str(self, _v: &str) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_bytes(self, _v: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_none(self) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, _v: &T) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_unit(self) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_unit_variant(self, _name: &'static str, _idx: u32, _variant: &'static str) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<(), Self::Error> {
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

// SerializeStruct — the important one. Captures field pointers.
impl<'a> ser::SerializeStruct for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        let ptr_id = ptr_to_id(value);
        let name = self.full_name(key);

        // Record this field's pointer ID and name.
        self.names.insert(ptr_id, name.clone());

        // Recurse into nested structs: temporarily set prefix, serialize, restore.
        let prev_prefix = std::mem::replace(&mut self.prefix, name);
        value.serialize(&mut **self).ok();
        self.prefix = prev_prefix;

        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

// No-op implementations for the other SerializeX traits.
impl<'a> ser::SerializeSeq for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl<'a> ser::SerializeTuple for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl<'a> ser::SerializeTupleStruct for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl<'a> ser::SerializeTupleVariant for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl<'a> ser::SerializeMap for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, _key: &T) -> Result<(), Self::Error> { Ok(()) }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

impl<'a> ser::SerializeStructVariant for &'a mut CatalogIndexer {
    type Ok = ();
    type Error = IndexerError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _key: &'static str, _value: &T) -> Result<(), Self::Error> { Ok(()) }
    fn end(self) -> Result<(), Self::Error> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::MemoryDataset;
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
}
