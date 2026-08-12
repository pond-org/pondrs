//! Type-erased dynamic steps.
use pondrs::{Node, PondError, Step, DynSteps};
use pondrs::datasets::{CellDataset, Param};
use pondrs::runners::{Runner, SequentialRunner};

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let steps: DynSteps<'_> = vec![Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32| (a,),
    }
    .boxed()];
    let _: Result<(), PondError> = SequentialRunner.run(&steps, &(), &(), &());
}
