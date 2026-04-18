//! Split and Join nodes for fan-out/fan-in patterns with TemplatedCatalog.

use std::prelude::v1::*;
use std::collections::HashMap;

use crate::datasets::{Dataset, TemplatedCatalog};
use crate::error::PondError;

use super::traits::{DatasetEvent, DatasetRef, StepInfo, RunnableStep};

/// A leaf node that distributes a `HashMap` across a `TemplatedCatalog`.
///
/// Loads a `HashMap<String, T>` from the input dataset, then saves each value
/// to the corresponding entry's dataset (selected by the `field` accessor).
/// Errors at runtime if the HashMap keys don't match the catalog keys.
pub struct Split<'a, Input, S, D, T>
where
    Input: Dataset<LoadItem = HashMap<String, T>> + Send + Sync,
    D: Dataset<SaveItem = T> + Send + Sync,
{
    pub name: &'static str,
    pub input: &'a Input,
    pub catalog: &'a TemplatedCatalog<S>,
    pub field: fn(&S) -> &D,
}

impl<Input, S, D, T> StepInfo for Split<'_, Input, S, D, T>
where
    Input: Dataset<LoadItem = HashMap<String, T>> + Send + Sync,
    D: Dataset<SaveItem = T> + Send + Sync,
    S: Send + Sync,
    T: Send + Sync,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn type_string(&self) -> &'static str {
        core::any::type_name::<Self>()
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn StepInfo)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.input));
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for (_, entry) in self.catalog.iter() {
            f(&DatasetRef::from_ref((self.field)(entry)));
        }
    }
}

impl<Input, S, D, T, E> RunnableStep<E> for Split<'_, Input, S, D, T>
where
    Input: Dataset<LoadItem = HashMap<String, T>> + Send + Sync,
    D: Dataset<SaveItem = T> + Send + Sync,
    S: Send + Sync,
    T: Send + Sync,
    E: From<PondError>,
    PondError: From<Input::Error> + From<D::Error>,
{
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        // Load the input HashMap.
        let input_ref = DatasetRef::from_ref(self.input);
        on_event(&input_ref, DatasetEvent::BeforeLoad);
        let mut map = self.input.load().map_err(|e| E::from(PondError::from(e)))?;
        on_event(&input_ref, DatasetEvent::AfterLoad);

        // Validate keys match.
        let mut expected: Vec<String> = self.catalog.keys().to_vec();
        let mut actual: Vec<String> = map.keys().cloned().collect();
        expected.sort();
        actual.sort();
        if expected != actual {
            return Err(E::from(PondError::KeyMismatch { expected, actual }));
        }

        // Distribute values to catalog entries.
        for (key, entry) in self.catalog.iter() {
            let value = map.remove(key).expect("key validated above");
            let ds = (self.field)(entry);
            let ds_ref = DatasetRef::from_ref(ds);
            on_event(&ds_ref, DatasetEvent::BeforeSave);
            ds.save(value).map_err(|e| E::from(PondError::from(e)))?;
            on_event(&ds_ref, DatasetEvent::AfterSave);
        }

        Ok(())
    }

    fn for_each_child_step<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {}

    fn as_pipeline_info(&self) -> &dyn StepInfo { self }
}

/// A leaf node that collects values from a `TemplatedCatalog` into a `HashMap`.
///
/// Loads a value from each catalog entry's dataset (selected by the `field`
/// accessor), collects them into a `HashMap<String, T>`, and saves to the
/// output dataset.
pub struct Join<'a, S, D, Output, T>
where
    D: Dataset<LoadItem = T> + Send + Sync,
    Output: Dataset<SaveItem = HashMap<String, T>> + Send + Sync,
{
    pub name: &'static str,
    pub catalog: &'a TemplatedCatalog<S>,
    pub field: fn(&S) -> &D,
    pub output: &'a Output,
}

impl<S, D, Output, T> StepInfo for Join<'_, S, D, Output, T>
where
    D: Dataset<LoadItem = T> + Send + Sync,
    Output: Dataset<SaveItem = HashMap<String, T>> + Send + Sync,
    S: Send + Sync,
    T: Send + Sync,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn type_string(&self) -> &'static str {
        core::any::type_name::<Self>()
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn StepInfo)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for (_, entry) in self.catalog.iter() {
            f(&DatasetRef::from_ref((self.field)(entry)));
        }
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.output));
    }
}

impl<S, D, Output, T, E> RunnableStep<E> for Join<'_, S, D, Output, T>
where
    D: Dataset<LoadItem = T> + Send + Sync,
    Output: Dataset<SaveItem = HashMap<String, T>> + Send + Sync,
    S: Send + Sync,
    T: Send + Sync,
    E: From<PondError>,
    PondError: From<D::Error> + From<Output::Error>,
{
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        // Load from each catalog entry.
        let mut map = HashMap::with_capacity(self.catalog.len());
        for (key, entry) in self.catalog.iter() {
            let ds = (self.field)(entry);
            let ds_ref = DatasetRef::from_ref(ds);
            on_event(&ds_ref, DatasetEvent::BeforeLoad);
            let value = ds.load().map_err(|e| E::from(PondError::from(e)))?;
            on_event(&ds_ref, DatasetEvent::AfterLoad);
            map.insert(key.to_string(), value);
        }

        // Save the collected HashMap.
        let output_ref = DatasetRef::from_ref(self.output);
        on_event(&output_ref, DatasetEvent::BeforeSave);
        self.output.save(map).map_err(|e| E::from(PondError::from(e)))?;
        on_event(&output_ref, DatasetEvent::AfterSave);

        Ok(())
    }

    fn for_each_child_step<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {}

    fn as_pipeline_info(&self) -> &dyn StepInfo { self }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::MemoryDataset;
    use crate::pipeline::{PipelineInfo, StepVec, Node, ptr_to_id};

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct ItemCatalog {
        raw: MemoryDataset<i32>,
        processed: MemoryDataset<i32>,
    }

    fn make_catalog() -> TemplatedCatalog<ItemCatalog> {
        let yaml = r#"
template:
  raw: {}
  processed: {}
names: [alpha, beta]
"#;
        serde_yaml::from_str(yaml).unwrap()
    }

    // ── Split tests ─────────────────────────────────────────────────────

    #[test]
    fn split_for_each_input_reports_single_input() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let split = Split {
            name: "split",
            input: &source,
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.raw,
        };

        let mut count = 0;
        split.for_each_input(&mut |_| count += 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn split_for_each_output_reports_catalog_entries() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let split = Split {
            name: "split",
            input: &source,
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.raw,
        };

        let mut ids = Vec::new();
        split.for_each_output(&mut |d| ids.push(d.id));
        assert_eq!(ids.len(), 2);
        // IDs should match the actual dataset addresses in the catalog.
        let expected: Vec<usize> = catalog.iter()
            .map(|(_, item)| ptr_to_id(&item.raw))
            .collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn split_call_distributes_values() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let mut input_map = HashMap::new();
        input_map.insert("alpha".to_string(), 10);
        input_map.insert("beta".to_string(), 20);
        source.save(input_map).unwrap();

        let catalog = make_catalog();

        let split = Split {
            name: "split",
            input: &source,
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.raw,
        };

        let result: Result<(), PondError> = split.call(&mut |_, _| {});
        assert!(result.is_ok());

        assert_eq!(catalog.get("alpha").unwrap().raw.load().unwrap(), 10);
        assert_eq!(catalog.get("beta").unwrap().raw.load().unwrap(), 20);
    }

    #[test]
    fn split_call_errors_on_key_mismatch() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let mut input_map = HashMap::new();
        input_map.insert("alpha".to_string(), 10);
        input_map.insert("gamma".to_string(), 30); // wrong key
        source.save(input_map).unwrap();

        let catalog = make_catalog();

        let split = Split {
            name: "split",
            input: &source,
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.raw,
        };

        let result: Result<(), PondError> = split.call(&mut |_, _| {});
        assert!(matches!(result, Err(PondError::KeyMismatch { .. })));
    }

    // ── Join tests ──────────────────────────────────────────────────────

    #[test]
    fn join_for_each_input_reports_catalog_entries() {
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let join = Join {
            name: "join",
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.processed,
            output: &result_ds,
        };

        let mut count = 0;
        join.for_each_input(&mut |_| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn join_for_each_output_reports_single_output() {
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let join = Join {
            name: "join",
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.processed,
            output: &result_ds,
        };

        let mut count = 0;
        join.for_each_output(&mut |_| count += 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn join_call_collects_values() {
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        // Pre-populate the catalog datasets.
        catalog.get("alpha").unwrap().processed.save(100).unwrap();
        catalog.get("beta").unwrap().processed.save(200).unwrap();

        let join = Join {
            name: "join",
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.processed,
            output: &result_ds,
        };

        let result: Result<(), PondError> = join.call(&mut |_, _| {});
        assert!(result.is_ok());

        let output = result_ds.load().unwrap();
        assert_eq!(output.get("alpha"), Some(&100));
        assert_eq!(output.get("beta"), Some(&200));
    }

    // ── Integration: Split → process → Join ─────────────────────────────

    #[test]
    fn split_process_join_roundtrip() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        // Set up input.
        let mut input_map = HashMap::new();
        input_map.insert("alpha".to_string(), 5);
        input_map.insert("beta".to_string(), 10);
        source.save(input_map).unwrap();

        let split = Split {
            name: "split",
            input: &source,
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.raw,
        };

        let join = Join {
            name: "join",
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.processed,
            output: &result_ds,
        };

        // Execute split, then process, then join.
        let noop = &mut |_: &DatasetRef<'_>, _: DatasetEvent| {};
        RunnableStep::<PondError>::call(&split, noop).unwrap();
        for (_, item) in catalog.iter() {
            let v = item.raw.load().unwrap();
            item.processed.save(v * 2).unwrap();
        }
        RunnableStep::<PondError>::call(&join, noop).unwrap();

        let output = result_ds.load().unwrap();
        assert_eq!(output.get("alpha"), Some(&10));
        assert_eq!(output.get("beta"), Some(&20));
    }

    // ── Check validation ────────────────────────────────────────────────

    #[test]
    fn check_valid_split_join_pipeline() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();
        let alpha = catalog.get("alpha").unwrap();
        let beta = catalog.get("beta").unwrap();

        let pipeline = (
            Split {
                name: "split",
                input: &source,
                catalog: &catalog,
                field: |s: &ItemCatalog| &s.raw,
            },
            Node { name: "proc_a", func: |x: i32| (x,), input: (&alpha.raw,), output: (&alpha.processed,) },
            Node { name: "proc_b", func: |x: i32| (x,), input: (&beta.raw,), output: (&beta.processed,) },
            Join {
                name: "join",
                catalog: &catalog,
                field: |s: &ItemCatalog| &s.processed,
                output: &result_ds,
            },
        );

        assert!(pipeline.check().is_ok());
    }

    #[test]
    fn check_with_step_vec() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let mut pipeline: StepVec<PondError> = vec![
            Split {
                name: "split",
                input: &source,
                catalog: &catalog,
                field: |s: &ItemCatalog| &s.raw,
            }.boxed(),
        ];
        for (_, item) in catalog.iter() {
            pipeline.push(Node {
                name: "process",
                func: |x: i32| (x,),
                input: (&item.raw,),
                output: (&item.processed,),
            }.boxed());
        }
        pipeline.push(Join {
            name: "join",
            catalog: &catalog,
            field: |s: &ItemCatalog| &s.processed,
            output: &result_ds,
        }.boxed());

        assert!(pipeline.check().is_ok());
    }

    #[test]
    fn check_catches_wrong_order() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        // Join before Split — the join inputs haven't been produced yet.
        let pipeline = (
            Join {
                name: "join",
                catalog: &catalog,
                field: |s: &ItemCatalog| &s.raw,
                output: &result_ds,
            },
            Split {
                name: "split",
                input: &source,
                catalog: &catalog,
                field: |s: &ItemCatalog| &s.raw,
            },
        );

        assert!(pipeline.check().is_err());
    }
}
