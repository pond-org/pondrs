//! Node function returns a bare value instead of a one-element tuple.
use pondrs::Node;
use pondrs::datasets::{CellDataset, Param};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let _n = Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32| a,
    };
}
