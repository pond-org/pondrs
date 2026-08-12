//! A steps tuple contains something that is not a pipeline step.
use pondrs::{Node, PondError};
use pondrs::datasets::{CellDataset, Param};
use pondrs::runners::{Runner, SequentialRunner};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let pipe = (
        Node {
            name: "n",
            input: (&p,),
            output: (&out,),
            func: |a: i32| (a,),
        },
        42u8,
    );
    let _: Result<(), PondError> = SequentialRunner.run(&pipe, &(), &(), &());
}
