//! Pipeline error type does not implement From<PondError>.
use pondrs::Node;
use pondrs::datasets::{CellDataset, Param};
use pondrs::runners::{Runner, SequentialRunner};

#[derive(Debug)]
struct MyErr;

impl std::fmt::Display for MyErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MyErr")
    }
}

fn main() {
    let p = Param(1i32);
    let out = CellDataset::<i32>::new();
    let pipe = (Node {
        name: "n",
        input: (&p,),
        output: (&out,),
        func: |a: i32| (a,),
    },);
    let _: Result<(), MyErr> = SequentialRunner.run(&pipe, &(), &(), &());
}
