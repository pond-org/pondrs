//! The legacy field order still compiles.
use pondrs::{Node, PondError};
use pondrs::datasets::{CellDataset, Param};
use pondrs::runners::{Runner, SequentialRunner};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let pipe = (Node {
        name: "n",
        func: |a: i32| (a,),
        input: (&p,),
        output: (&out,),
    },);
    let _: Result<(), PondError> = SequentialRunner.run(&pipe, &(), &(), &());
}
