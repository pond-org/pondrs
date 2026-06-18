//! EachField port adapter for fan-out/fan-in patterns with TemplatedCatalog.

use std::prelude::v1::*;
use std::collections::HashMap;

use crate::datasets::{Dataset, TemplatedCatalog};
use crate::error::PondError;

use crate::hooks::{HookAbort, HookControl};
use super::traits::{DatasetEvent, DatasetRef, DatasetInput, DatasetOutput};

/// A port that fans out to / fans in from all entries of a [`TemplatedCatalog`].
///
/// Used as an element in a [`Node`](super::Node)'s input or output tuple:
/// - As **output** (fan-out): distributes a `HashMap<String, T>` to per-entry datasets.
/// - As **input** (fan-in): loads from each per-entry dataset into a `HashMap<String, T>`.
pub struct EachField<'a, S, D> {
    pub catalog: &'a TemplatedCatalog<S>,
    pub field: fn(&S) -> &D,
}

impl<S, D> DatasetInput for EachField<'_, S, D>
where
    S: Send + Sync,
    D: Dataset + Send + Sync,
    D::LoadItem: Send + Sync + 'static,
    PondError: From<D::Error>,
{
    type Item = HashMap<String, D::LoadItem>;

    fn load_input(
        &self,
        on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>,
    ) -> Result<Self::Item, PondError> {
        let mut map = HashMap::with_capacity(self.catalog.len());
        for (key, entry) in self.catalog.iter() {
            let ds = (self.field)(entry);
            let ds_ref = DatasetRef::from_ref(ds);
            on_event(&ds_ref, DatasetEvent::BeforeLoad)?;
            let value = ds.load()?;
            on_event(&ds_ref, DatasetEvent::AfterLoad(&value))?;
            map.insert(key.to_string(), value);
        }
        Ok(map)
    }

    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for (_, entry) in self.catalog.iter() {
            f(&DatasetRef::from_ref((self.field)(entry)));
        }
    }
}

impl<S, D> DatasetOutput for EachField<'_, S, D>
where
    S: Send + Sync,
    D: Dataset + Send + Sync,
    D::SaveItem: Send + Sync + 'static,
    PondError: From<D::Error>,
{
    type Item = HashMap<String, D::SaveItem>;

    fn save_output(
        &self,
        mut value: Self::Item,
        on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>,
    ) -> Result<(), PondError> {
        let mut expected: Vec<String> = self.catalog.keys().to_vec();
        let mut actual: Vec<String> = value.keys().cloned().collect();
        expected.sort();
        actual.sort();
        if expected != actual {
            return Err(PondError::KeyMismatch { expected, actual });
        }

        for (key, entry) in self.catalog.iter() {
            let item = value.remove(key).expect("key validated above");
            let ds = (self.field)(entry);
            let ds_ref = DatasetRef::from_ref(ds);
            let control = on_event(&ds_ref, DatasetEvent::BeforeSave(&item))?;
            if control != HookControl::Skip {
                ds.save(item)?;
                on_event(&ds_ref, DatasetEvent::AfterSave)?;
            }
        }
        Ok(())
    }

    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for (_, entry) in self.catalog.iter() {
            f(&DatasetRef::from_ref((self.field)(entry)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::MemoryDataset;
    use crate::pipeline::{PipelineInfo, RunnableStep, StepInfo, StepVec, Node, ptr_to_id};
    use crate::pipeline::traits::LeafStep;

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

    // ── Fan-out (split) tests ───────────────────────────────────────────

    #[test]
    fn split_for_each_input_reports_single_input() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let split = Node {
            name: "split",
            func: |m: HashMap<String, i32>| (m,),
            input: (&source,),
            output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
        };

        let mut count = 0;
        split.for_each_input(&mut |_| count += 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn split_for_each_output_reports_catalog_entries() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let split = Node {
            name: "split",
            func: |m: HashMap<String, i32>| (m,),
            input: (&source,),
            output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
        };

        let mut ids = Vec::new();
        split.for_each_output(&mut |d| ids.push(d.id));
        assert_eq!(ids.len(), 2);
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

        let split = Node {
            name: "split",
            func: |m: HashMap<String, i32>| (m,),
            input: (&source,),
            output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
        };

        let result: Result<(), PondError> = split.call(&mut |_, _| Ok(HookControl::Continue));
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

        let split = Node {
            name: "split",
            func: |m: HashMap<String, i32>| (m,),
            input: (&source,),
            output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
        };

        let result: Result<(), PondError> = split.call(&mut |_, _| Ok(HookControl::Continue));
        assert!(matches!(result, Err(PondError::KeyMismatch { .. })));
    }

    // ── Fan-in (join) tests ─────────────────────────────────────────────

    #[test]
    fn join_for_each_input_reports_catalog_entries() {
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let join = Node {
            name: "join",
            func: |m: HashMap<String, i32>| (m,),
            input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.processed },),
            output: (&result_ds,),
        };

        let mut count = 0;
        join.for_each_input(&mut |_| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn join_for_each_output_reports_single_output() {
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let join = Node {
            name: "join",
            func: |m: HashMap<String, i32>| (m,),
            input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.processed },),
            output: (&result_ds,),
        };

        let mut count = 0;
        join.for_each_output(&mut |_| count += 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn join_call_collects_values() {
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        catalog.get("alpha").unwrap().processed.save(100).unwrap();
        catalog.get("beta").unwrap().processed.save(200).unwrap();

        let join = Node {
            name: "join",
            func: |m: HashMap<String, i32>| (m,),
            input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.processed },),
            output: (&result_ds,),
        };

        let result: Result<(), PondError> = join.call(&mut |_, _| Ok(HookControl::Continue));
        assert!(result.is_ok());

        let output = result_ds.load().unwrap();
        assert_eq!(output.get("alpha"), Some(&100));
        assert_eq!(output.get("beta"), Some(&200));
    }

    // ── Integration: split → process → join ─────────────────────────────

    #[test]
    fn split_process_join_roundtrip() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        let mut input_map = HashMap::new();
        input_map.insert("alpha".to_string(), 5);
        input_map.insert("beta".to_string(), 10);
        source.save(input_map).unwrap();

        let split = Node {
            name: "split",
            func: |m: HashMap<String, i32>| (m,),
            input: (&source,),
            output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
        };

        let join = Node {
            name: "join",
            func: |m: HashMap<String, i32>| (m,),
            input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.processed },),
            output: (&result_ds,),
        };

        let noop = &mut |_: &DatasetRef<'_>, _: DatasetEvent| Ok(HookControl::Continue);
        LeafStep::<PondError>::call(&split, noop).unwrap();
        for (_, item) in catalog.iter() {
            let v = item.raw.load().unwrap();
            item.processed.save(v * 2).unwrap();
        }
        let noop = &mut |_: &DatasetRef<'_>, _: DatasetEvent| Ok(HookControl::Continue);
        LeafStep::<PondError>::call(&join, noop).unwrap();

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
            Node {
                name: "split",
                func: |m: HashMap<String, i32>| (m,),
                input: (&source,),
                output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
            },
            Node { name: "proc_a", func: |x: i32| (x,), input: (&alpha.raw,), output: (&alpha.processed,) },
            Node { name: "proc_b", func: |x: i32| (x,), input: (&beta.raw,), output: (&beta.processed,) },
            Node {
                name: "join",
                func: |m: HashMap<String, i32>| (m,),
                input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.processed },),
                output: (&result_ds,),
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
            Node {
                name: "split",
                func: |m: HashMap<String, i32>| (m,),
                input: (&source,),
                output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
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
        pipeline.push(Node {
            name: "join",
            func: |m: HashMap<String, i32>| (m,),
            input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.processed },),
            output: (&result_ds,),
        }.boxed());

        assert!(pipeline.check().is_ok());
    }

    #[test]
    fn check_catches_wrong_order() {
        let source = MemoryDataset::<HashMap<String, i32>>::new();
        let result_ds = MemoryDataset::<HashMap<String, i32>>::new();
        let catalog = make_catalog();

        // Join before split — the join inputs haven't been produced yet.
        let pipeline = (
            Node {
                name: "join",
                func: |m: HashMap<String, i32>| (m,),
                input: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
                output: (&result_ds,),
            },
            Node {
                name: "split",
                func: |m: HashMap<String, i32>| (m,),
                input: (&source,),
                output: (EachField { catalog: &catalog, field: |s: &ItemCatalog| &s.raw },),
            },
        );

        assert!(pipeline.check().is_err());
    }
}
