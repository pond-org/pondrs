//! `func` is not callable.
use pondrs::Node;
use pondrs::datasets::{CellDataset, Param};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let _n = Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: 42u8,
    };
}
