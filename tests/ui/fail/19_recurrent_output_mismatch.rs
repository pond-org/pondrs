//! RecurrentNode function returns a type its `output` projection cannot store.
//! The diagnostic must still land on the closure, as for a plain `Node`.
use pondrs::RecurrentNode;
use pondrs::datasets::MemoryDataset;

fn main() {
    let init = MemoryDataset::<i32>::new();
    let state: Vec<MemoryDataset<i32>> = vec![MemoryDataset::new()];
    let _ = RecurrentNode {
        name: "train",
        state: &state,
        init: &init,
        input: |prev, _cur| (prev,),
        output: |cur| (cur,),
        func: |x: i32| (x.to_string(),),
    }
    .build();
}
