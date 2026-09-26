//! `expand_indexed`: write a `Vec<S>` catalog field as a count and a template.

use std::prelude::v1::*;

use serde::de::{self, Deserialize, DeserializeOwned, Deserializer};

use super::templated::replace_in_value;

/// `deserialize_with` helper that expands `{count, template}` into a `Vec<S>`.
///
/// A `Vec<S>` catalog field gives each element its own address — so each can
/// be a distinct node's output — and the catalog indexer names the elements
/// `field.0`, `field.1`, … Writing 100 of them out in YAML is the only chore;
/// this removes it:
///
/// ```rust,ignore
/// #[derive(Serialize, Deserialize)]
/// struct Catalog {
///     #[serde(deserialize_with = "pondrs::datasets::expand_indexed")]
///     epochs: Vec<EpochSlot>,
///     init_epoch: EpochSlot,
/// }
/// ```
///
/// ```yaml
/// epochs:
///   count: 100
///   template:
///     weights: { path: "ckpt/epoch_{i}.json" }
///     lr: 0.001
/// ```
///
/// Element `k` is the template with every `{i}` in its string values replaced
/// by `k`. Substitution is over the YAML value, not the struct shape, so the
/// same works for a vector of bare datasets (`template: { path: "ckpt/{i}.json" }`).
/// An optional `placeholder` key picks a different name — needed when one
/// expanded vector sits inside another's template, as with
/// [`TemplatedCatalog`](super::TemplatedCatalog).
///
/// A plain YAML sequence is accepted too, so a catalog serialized back out (as
/// `App::with_cli` does to apply overrides) deserializes again unchanged.
///
/// Since the length now lives in the catalog, `--catalog epochs.count=50`
/// changes the iteration count of anything built over `cat.epochs`.
///
/// # Errors
///
/// Fails if the value is neither a sequence nor a `{count, template}` map, if
/// the map has unknown keys, or if an expanded element does not deserialize
/// into `S`.
pub fn expand_indexed<'de, D, S>(deserializer: D) -> Result<Vec<S>, D::Error>
where
    D: Deserializer<'de>,
    S: DeserializeOwned,
{
    let value = serde_yaml::Value::deserialize(deserializer)?;
    let map = match value {
        serde_yaml::Value::Sequence(_) => {
            return serde_yaml::from_value(value).map_err(de::Error::custom);
        }
        serde_yaml::Value::Mapping(map) => map,
        _ => {
            return Err(de::Error::custom(
                "expected a sequence or a map with 'count' and 'template'",
            ));
        }
    };

    let mut count: Option<usize> = None;
    let mut template: Option<serde_yaml::Value> = None;
    let mut placeholder: Option<String> = None;
    for (key, v) in map {
        let key = key
            .as_str()
            .ok_or_else(|| de::Error::custom("expected string keys"))?
            .to_string();
        match key.as_str() {
            "count" => count = Some(serde_yaml::from_value(v).map_err(de::Error::custom)?),
            "template" => template = Some(v),
            "placeholder" => placeholder = Some(serde_yaml::from_value(v).map_err(de::Error::custom)?),
            other => {
                return Err(de::Error::unknown_field(other, &["count", "template", "placeholder"]));
            }
        }
    }
    let count = count.ok_or_else(|| de::Error::missing_field("count"))?;
    let template = template.ok_or_else(|| de::Error::missing_field("template"))?;
    let pattern = format!("{{{}}}", placeholder.as_deref().unwrap_or("i"));

    (0..count)
        .map(|i| {
            let mut value = template.clone();
            replace_in_value(&mut value, &pattern, &i.to_string());
            serde_yaml::from_value(value)
                .map_err(|e| de::Error::custom(format!("failed to expand template for element {i}: {e}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Slot {
        path: String,
        lr: f64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Catalog {
        #[serde(deserialize_with = "super::expand_indexed")]
        epochs: Vec<Slot>,
    }

    #[test]
    fn expands_count_and_template() {
        let yaml = r#"
epochs:
  count: 3
  template:
    path: "ckpt/epoch_{i}.json"
    lr: 0.5
"#;
        let cat: Catalog = serde_yaml::from_str(yaml).unwrap();
        let paths: Vec<&str> = cat.epochs.iter().map(|s| s.path.as_str()).collect();
        assert_eq!(paths, ["ckpt/epoch_0.json", "ckpt/epoch_1.json", "ckpt/epoch_2.json"]);
        assert!(cat.epochs.iter().all(|s| (s.lr - 0.5).abs() < f64::EPSILON));
    }

    #[test]
    fn accepts_a_plain_sequence_and_round_trips() {
        let yaml = r#"
epochs:
  count: 2
  template: { path: "p/{i}", lr: 1.0 }
"#;
        let cat: Catalog = serde_yaml::from_str(yaml).unwrap();
        // Serializing writes a plain sequence, which must read back the same.
        let value = serde_yaml::to_value(&cat).unwrap();
        assert!(value["epochs"].is_sequence());
        let back: Catalog = serde_yaml::from_value(value).unwrap();
        assert_eq!(back.epochs[1].path, "p/1");
    }

    #[test]
    fn bare_dataset_elements_and_custom_placeholder() {
        #[derive(Deserialize)]
        struct Bare {
            #[serde(deserialize_with = "super::expand_indexed")]
            items: Vec<crate::datasets::TextDataset>,
        }
        let yaml = r#"
items:
  placeholder: k
  count: 2
  template: { path: "out/{k}.txt" }
"#;
        let cat: Bare = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cat.items.len(), 2);
        assert_eq!(serde_yaml::to_value(&cat.items[1]).unwrap()["path"], "out/1.txt");
    }

    #[test]
    fn rejects_unknown_keys_and_missing_count() {
        let unknown = "epochs: { count: 1, template: { path: p, lr: 1.0 }, extra: 3 }";
        assert!(serde_yaml::from_str::<Catalog>(unknown).is_err());
        let missing = "epochs: { template: { path: p, lr: 1.0 } }";
        assert!(serde_yaml::from_str::<Catalog>(missing).is_err());
    }
}
