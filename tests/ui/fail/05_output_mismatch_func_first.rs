//! Node function returns a type the output dataset cannot save.
//! `func` declared before `input`/`output`.
use pondrs::Node;
use pondrs::datasets::{MemoryDataset, Param};

fn main() {
    let p = Param(1i32);
    let out = MemoryDataset::<String>::new();
    let _n = Node {
        name: "n",
        func: |a: i32| (a,),
        input: (&p,),
        output: (&out,),
    };
}
