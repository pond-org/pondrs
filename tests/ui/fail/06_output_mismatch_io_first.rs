//! Node function returns a type the output dataset cannot save.
//! `input`/`output` declared before `func` (the recommended order).
use pondrs::Node;
use pondrs::datasets::{MemoryDataset, Param};

fn main() {
    let p = Param(1i32);
    let out = MemoryDataset::<String>::new();
    let _n = Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32| (a,),
    };
}
