//! EachField fan-out with a value type the per-entry datasets cannot save.
use std::collections::HashMap;

use pondrs::{EachField, Node, TemplatedCatalog};
use pondrs::datasets::MemoryDataset;

#[derive(serde::Serialize, serde::Deserialize)]
struct Entry {
    raw: MemoryDataset<i32>,
}

fn main() {
    let catalog: TemplatedCatalog<Entry> =
        serde_yaml::from_str("template:\n  raw: {}\nnames: [a]\n").unwrap();
    let src = MemoryDataset::<HashMap<String, String>>::new();
    let _n = Node {
        name: "n",
        input: (&src,),
        output: (EachField { catalog: &catalog, field: |e: &Entry| &e.raw },),
        func: |m: HashMap<String, String>| (m,),
    };
}
