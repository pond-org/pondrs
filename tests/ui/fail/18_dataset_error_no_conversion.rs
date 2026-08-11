// A custom dataset whose `Error` does not convert into `PondError` cannot be
// used as a node input or output.
use pondrs::{Dataset, Node};

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
    let _n = Node {
        name: "n",
        input: (&ds,),
        output: (),
        func: |a: i32| {
            let _ = a;
        },
    };
}
