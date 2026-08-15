// A custom dataset's `Error` must convert into the *pipeline* error type.
//
// The `Node { .. }` literal itself compiles — `Args` and the closure signature
// resolve without knowing the error type. The failure fires where the pipeline
// error type is named, exactly like `09_node_error_no_conversion.rs`. Here that
// type is `PondError`, which knows nothing about `MyErr`; the fix is a custom
// error enum with a `From<MyErr>` variant.
use pondrs::{Dataset, Node, PondError, Step};

#[derive(serde::Serialize)]
struct MyDataset;

struct MyErr;

impl Dataset for MyDataset {
    type LoadItem = i32;
    type SaveItem = i32;
    type Error = MyErr;
    fn load(&self) -> Result<i32, MyErr> {
        Ok(0)
    }
    fn save(&self, _output: i32) -> Result<(), MyErr> {
        Ok(())
    }
}

fn main() {
    let ds = MyDataset;
    let n = Node {
        name: "n",
        input: (&ds,),
        output: (),
        func: |a: i32| {
            let _ = a;
        },
    };
    let _step: &dyn Step<PondError> = &n;
}
