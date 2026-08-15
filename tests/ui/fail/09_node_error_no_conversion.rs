//! Node returns Result with an error type the pipeline error cannot absorb.
use pondrs::{Node, PondError, Step};
use pondrs::datasets::{CellDataset, Param};

#[derive(Debug)]
struct MyErr;

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let n = Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32| -> Result<(i32,), MyErr> { Ok((a,)) },
    };
    let _step: &dyn Step<PondError> = &n;
}
