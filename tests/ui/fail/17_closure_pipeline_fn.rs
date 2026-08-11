//! Pipeline functions must be named `fn`s with an explicit lifetime.
//! A closure desugars into two independent lifetimes and cannot borrow the catalog.
use pondrs::{App, Node, PondError};
use pondrs::datasets::{MemoryDataset, Param};

#[derive(serde::Serialize)]
struct Catalog {
    a: MemoryDataset<i32>,
}

#[derive(serde::Serialize)]
struct Params {
    x: Param<i32>,
}

fn main() {
    let app = App::new(
        Catalog { a: MemoryDataset::new() },
        Params { x: Param(1) },
    );
    let _: Result<(), PondError> = app.execute(|cat: &Catalog, params: &Params| {
        (Node {
            name: "n",
            input: (&params.x,),
            output: (&cat.a,),
            func: |v: i32| (v,),
        },)
    });
}
