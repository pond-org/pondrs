//! A node returning Result with a convertible error type.
use pondrs::{Node, PondError};
use pondrs::datasets::{CellDataset, Param};
use pondrs::runners::{Runner, SequentialRunner};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let pipe = (Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32| -> Result<(i32,), PondError> { Ok((a,)) },
    },);
    let _: Result<(), PondError> = SequentialRunner.run(&pipe, &(), &(), &());
}
