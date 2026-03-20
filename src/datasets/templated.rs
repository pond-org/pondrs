//! Templated catalog: a collection of named sub-catalog instances expanded from a YAML template.

use std::prelude::v1::*;
use std::collections::HashMap;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

/// A collection of named sub-catalog structs expanded from a YAML template.
///
/// Deserializes from YAML with the structure:
/// ```yaml
/// placeholder: "city"      # optional, defaults to "name"
/// template:
///   raw: { path: "data/{city}/raw.csv" }
///   processed: { path: "data/{city}/processed.csv" }
/// names: [london, paris]
/// ```
///
/// Each name produces an instance of `S` with all occurrences of `{placeholder}`
/// in string values replaced by that name.
#[derive(Debug)]
pub struct TemplatedCatalog<S> {
    names: Vec<String>,
    items: HashMap<String, S>,
}

impl<S> TemplatedCatalog<S> {
    /// Iterate over entries in name-insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &S)> {
        self.names.iter().map(move |name| {
            (name.as_str(), self.items.get(name).expect("name must exist in items"))
        })
    }

    /// Look up an entry by name.
    pub fn get(&self, name: &str) -> Option<&S> {
        self.items.get(name)
    }

    /// The ordered list of names.
    pub fn keys(&self) -> &[String] {
        &self.names
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

// Serialize as a map of name -> S, so the catalog indexer recurses into entries.
impl<S: Serialize> Serialize for TemplatedCatalog<S> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut map = serializer.serialize_map(Some(self.names.len()))?;
        for name in &self.names {
            let item = self.items.get(name).expect("name must exist in items");
            map.serialize_entry(name, item)?;
        }
        map.end()
    }
}

/// Recursively replace all occurrences of `pattern` in string values within a serde_yaml::Value.
fn replace_in_value(value: &mut serde_yaml::Value, pattern: &str, replacement: &str) {
    match value {
        serde_yaml::Value::String(s) => {
            if s.contains(pattern) {
                *s = s.replace(pattern, replacement);
            }
        }
        serde_yaml::Value::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                replace_in_value(v, pattern, replacement);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for v in seq.iter_mut() {
                replace_in_value(v, pattern, replacement);
            }
        }
        _ => {}
    }
}

impl<'de, S: serde::de::DeserializeOwned> serde::Deserialize<'de> for TemplatedCatalog<S> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(TemplatedCatalogVisitor::<S>(std::marker::PhantomData))
    }
}

struct TemplatedCatalogVisitor<S>(std::marker::PhantomData<S>);

impl<'de, S: serde::de::DeserializeOwned> Visitor<'de> for TemplatedCatalogVisitor<S> {
    type Value = TemplatedCatalog<S>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a map with 'template', 'names', and optional 'placeholder' fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut placeholder: Option<String> = None;
        let mut template: Option<serde_yaml::Value> = None;
        let mut names: Option<Vec<String>> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "placeholder" => {
                    placeholder = Some(map.next_value()?);
                }
                "template" => {
                    template = Some(map.next_value()?);
                }
                "names" => {
                    names = Some(map.next_value()?);
                }
                other => {
                    return Err(de::Error::unknown_field(other, &["placeholder", "template", "names"]));
                }
            }
        }

        let template = template.ok_or_else(|| de::Error::missing_field("template"))?;
        let names = names.ok_or_else(|| de::Error::missing_field("names"))?;
        let placeholder_str = placeholder.unwrap_or_else(|| "name".to_string());
        let pattern = format!("{{{}}}", placeholder_str);

        let mut items = HashMap::with_capacity(names.len());
        for name in &names {
            let mut value = template.clone();
            replace_in_value(&mut value, &pattern, name);
            let instance: S = serde_yaml::from_value(value)
                .map_err(|e| de::Error::custom(format!("failed to expand template for '{}': {}", name, e)))?;
            items.insert(name.clone(), instance);
        }

        Ok(TemplatedCatalog { names, items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::MemoryDataset;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct ItemCatalog {
        raw: MemoryDataset<i32>,
        processed: MemoryDataset<i32>,
    }

    #[test]
    fn deserialize_default_placeholder() {
        let yaml = r#"
template:
  raw: {}
  processed: {}
names: [alpha, beta]
"#;
        let tc: TemplatedCatalog<ItemCatalog> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tc.len(), 2);
        assert_eq!(tc.keys(), &["alpha", "beta"]);
        assert!(tc.get("alpha").is_some());
        assert!(tc.get("beta").is_some());
        assert!(tc.get("gamma").is_none());
    }

    #[test]
    fn deserialize_custom_placeholder() {
        let yaml = r#"
placeholder: "city"
template:
  raw: {}
  processed: {}
names: [london, paris]
"#;
        let tc: TemplatedCatalog<ItemCatalog> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tc.len(), 2);
        assert_eq!(tc.keys(), &["london", "paris"]);
    }

    #[test]
    fn iter_preserves_order() {
        let yaml = r#"
template:
  raw: {}
  processed: {}
names: [charlie, alpha, bravo]
"#;
        let tc: TemplatedCatalog<ItemCatalog> = serde_yaml::from_str(yaml).unwrap();
        let order: Vec<&str> = tc.iter().map(|(k, _)| k).collect();
        assert_eq!(order, vec!["charlie", "alpha", "bravo"]);
    }

    #[test]
    fn serialize_as_map() {
        let yaml = r#"
template:
  raw: {}
  processed: {}
names: [alpha, beta]
"#;
        let tc: TemplatedCatalog<ItemCatalog> = serde_yaml::from_str(yaml).unwrap();
        let value: serde_yaml::Value = serde_yaml::to_value(&tc).unwrap();
        assert!(value.is_mapping());
        let mapping = value.as_mapping().unwrap();
        assert!(mapping.contains_key("alpha"));
        assert!(mapping.contains_key("beta"));
    }

    // Test with file-path-bearing datasets to verify placeholder substitution.
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct FileItemCatalog {
        path: String,
    }

    #[test]
    fn placeholder_substitution_in_strings() {
        let yaml = r#"
placeholder: "city"
template:
  path: "data/{city}/raw.csv"
names: [london, paris]
"#;
        let tc: TemplatedCatalog<FileItemCatalog> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tc.get("london").unwrap().path, "data/london/raw.csv");
        assert_eq!(tc.get("paris").unwrap().path, "data/paris/raw.csv");
    }

    // Test nested TemplatedCatalog.
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct OuterItem {
        inner: TemplatedCatalog<FileItemCatalog>,
    }

    #[test]
    fn nested_templated_catalog() {
        let yaml = r#"
placeholder: "city"
template:
  inner:
    placeholder: "metric"
    template:
      path: "data/{city}/{metric}/raw.csv"
    names: [temp, humidity]
names: [london, paris]
"#;
        let tc: TemplatedCatalog<OuterItem> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tc.len(), 2);
        let london = tc.get("london").unwrap();
        assert_eq!(london.inner.len(), 2);
        assert_eq!(london.inner.get("temp").unwrap().path, "data/london/temp/raw.csv");
        assert_eq!(london.inner.get("humidity").unwrap().path, "data/london/humidity/raw.csv");
        let paris = tc.get("paris").unwrap();
        assert_eq!(paris.inner.get("temp").unwrap().path, "data/paris/temp/raw.csv");
    }
}
