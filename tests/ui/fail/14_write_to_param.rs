//! Params are read-only: their SaveItem is uninhabited, so no node can write one.
use pondrs::Node;
use pondrs::datasets::Param;

fn main() {
    let p = Param(1i32);
    let _n = Node {
        name: "n",
        input: (),
        output: (&p,),
        func: || ((),),
    };
}
