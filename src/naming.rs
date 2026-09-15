//! Shared naming-convention helpers.
//!
//! The catalog indexer identifies dataset leaves by their *serde struct name*,
//! and the `no_std` pipeline checks predict the same decision from
//! [`core::any::type_name`]. Both go through [`is_leaf_type`] so the prediction
//! cannot drift from what the indexer actually does.

// CONVENTION: Leaf type detection via type name.
//
// In Rust, a struct and its first field share the same memory address.
// If the catalog indexer recursed into every struct, a dataset and its first
// field would both get recorded under different names but the same pointer ID —
// the last write wins, producing wrong or missing entries.
//
// To stop recursion at dataset/param boundaries the indexer inspects the serde
// struct name passed to `serialize_struct` / `serialize_newtype_struct`:
//   - Names ending with `"Dataset"` → treated as leaves (no field recursion)
//   - The name `"Param"` → treated as a leaf (newtype wrapper)
//   - All other names → recursed into (container structs, nested catalogs)
//
// IMPORTANT: User-defined dataset types MUST end with "Dataset" for this to
// work. Container structs (catalogs, param groups) MUST NOT end with "Dataset".
/// Returns true if a type ident indicates a Dataset or Param leaf type.
pub(crate) fn is_leaf_type(name: &str) -> bool {
    name.ends_with("Dataset") || name == "Param"
}

/// Extract the bare ident from a [`core::any::type_name`] string.
///
/// Generic arguments are truncated *first*: they contain `::` of their own and
/// would otherwise win the `rfind` below.
///
/// ```text
/// pondrs::datasets::memory::MemoryDataset<i32> -> MemoryDataset
/// pondrs::datasets::param::Param<f64>          -> Param
/// myapp::S3Store                               -> S3Store
/// ```
pub(crate) fn type_ident(type_name: &str) -> &str {
    let base = match type_name.find('<') {
        Some(i) => &type_name[..i],
        None => type_name,
    };
    match base.rfind("::") {
        Some(i) => &base[i + 2..],
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_ident_strips_path_and_generics() {
        assert_eq!(
            type_ident("pondrs::datasets::memory::MemoryDataset<i32>"),
            "MemoryDataset"
        );
        assert_eq!(type_ident("pondrs::datasets::param::Param<f64>"), "Param");
        assert_eq!(type_ident("myapp::S3Store"), "S3Store");
    }

    #[test]
    fn type_ident_handles_nested_generic_paths() {
        // The generic argument's `::` must not win over the outer path's.
        assert_eq!(
            type_ident("pondrs::datasets::param::Param<myapp::config::MyConfig>"),
            "Param"
        );
    }

    #[test]
    fn leaf_types_recognised() {
        assert!(is_leaf_type("MemoryDataset"));
        assert!(is_leaf_type("Param"));
        assert!(!is_leaf_type("S3Store"));
        assert!(!is_leaf_type("Catalog"));
    }
}
