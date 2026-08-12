//! Node function parameter type does not match the input dataset's LoadItem.
//! `input`/`output` declared before `func` (the recommended order).
use pondrs::Node;
use pondrs::datasets::{CellDataset, Param};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let _n = Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |s: String| (s.len() as i32,),
    };
}
