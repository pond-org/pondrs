//! Two inputs, but the node function takes one argument.
use pondrs::Node;
use pondrs::datasets::{CellDataset, Param};

fn main() {
    let p = Param(1i32);
    let q = Param(2i32);
    let out = CellDataset::<i32>::new();
    let _n = Node {
        name: "n",
        input: (&p, &q),
        output: (&out,),
        func: |a: i32| (a,),
    };
}
