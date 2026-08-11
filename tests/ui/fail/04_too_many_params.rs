//! One input, but the node function takes two arguments.
use pondrs::Node;
use pondrs::datasets::{CellDataset, Param};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let _n = Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32, b: i32| (a + b,),
    };
}
