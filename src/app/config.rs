//! YAML configuration loading and param override patching.

use std::prelude::v1::*;
use std::fs;

use serde::de::DeserializeOwned;

use crate::error::PondError;

/// Load a YAML file and parse it into a generic `serde_yaml::Value` tree.
pub fn load_yaml(path: &str) -> Result<serde_yaml::Value, PondError> {
    let contents = fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&contents)?;
    Ok(value)
}

/// Apply dot-notation key=value overrides to a YAML value tree.
///
/// Each override string must be in the form `key.subkey=value`.
/// Values are parsed as YAML scalars (auto-detecting numbers, bools, strings, null).
pub fn apply_overrides(value: &mut serde_yaml::Value, overrides: &[String]) {
    for entry in overrides {
        let (dotted_key, raw_val) = match entry.split_once('=') {
            Some(pair) => pair,
            None => {
                eprintln!("Warning: ignoring malformed override (expected KEY=VALUE): {entry}");
                continue;
            }
        };

        let parts: Vec<&str> = dotted_key.split('.').collect();

        // Navigate to the parent of the target key, then set the leaf.
        // We use indexing which creates missing keys as Null (serde_yaml behavior).
        let leaf = parts.last().unwrap();
        let mut current = &mut *value;
        for part in &parts[..parts.len() - 1] {
            current = &mut current[*part];
        }

        let parsed: serde_yaml::Value = serde_yaml::from_str(raw_val)
            .unwrap_or(serde_yaml::Value::String(raw_val.to_string()));
        current[*leaf] = parsed;
    }
}

/// Deserialize a `serde_yaml::Value` into a concrete config type.
pub fn deserialize_config<T: DeserializeOwned>(value: serde_yaml::Value) -> Result<T, PondError> {
    Ok(serde_yaml::from_value(value)?)
}
