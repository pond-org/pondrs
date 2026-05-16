use std::sync::{Arc, Mutex};

use serde::Serialize;

use pondrs::datasets::MemoryDataset;
use pondrs::error::PondError;
use pondrs::{Dataset, IntoTypedHook, Node, Runner, SequentialRunner, TypedHook};
use pondrs::pipeline::{DatasetRef, StepInfo};

#[derive(Serialize)]
struct Catalog {
    input_i32: MemoryDataset<i32>,
    mid_string: MemoryDataset<String>,
    output_i32: MemoryDataset<i32>,
}

#[derive(Serialize)]
struct Params;

#[derive(Default)]
struct Recorded {
    loaded: Vec<i32>,
    saved: Vec<i32>,
}

struct I32Recorder(Arc<Mutex<Recorded>>);

impl TypedHook<i32> for I32Recorder {
    fn after_load(&self, _n: &dyn StepInfo, _ds: &DatasetRef, value: &i32) {
        self.0.lock().unwrap().loaded.push(*value);
    }
    fn before_save(&self, _n: &dyn StepInfo, _ds: &DatasetRef, value: &i32) {
        self.0.lock().unwrap().saved.push(*value);
    }
}

#[test]
fn typed_hook_fires_only_for_matching_type() {
    let catalog = Catalog {
        input_i32: MemoryDataset::new(),
        mid_string: MemoryDataset::new(),
        output_i32: MemoryDataset::new(),
    };
    catalog.input_i32.save(42).unwrap();

    let params = Params;

    let pipe = (
        Node {
            name: "to_string",
            func: |v: i32| (format!("value={v}"),),
            input: (&catalog.input_i32,),
            output: (&catalog.mid_string,),
        },
        Node {
            name: "parse_back",
            func: |s: String| {
                let n: i32 = s.strip_prefix("value=").unwrap().parse().unwrap();
                (n * 2,)
            },
            input: (&catalog.mid_string,),
            output: (&catalog.output_i32,),
        },
    );

    let recorded = Arc::new(Mutex::new(Recorded::default()));
    let hooks = (I32Recorder(Arc::clone(&recorded)).typed(),);

    SequentialRunner
        .run::<PondError>(&pipe, &catalog, &params, &hooks)
        .unwrap();

    assert_eq!(catalog.output_i32.load().unwrap(), 84);

    let r = recorded.lock().unwrap();
    // after_load fires for i32 datasets only: 42 (input_i32 loaded by node1)
    // String loads are silently ignored by the typed adapter
    assert_eq!(r.loaded, vec![42]);
    // before_save fires for i32 datasets only: 84 (output_i32 saved by node2)
    assert_eq!(r.saved, vec![84]);
}
