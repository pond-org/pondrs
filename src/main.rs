use pondrs::datasets::{MemoryDataset, Param, PolarsDataset};
use pondrs::hooks::LoggingHook;
use pondrs::runners::{ParallelRunner, Runner, SequentialRunner};
use pondrs::{Node, Pipeline, Steps};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Catalog {
    a: MemoryDataset<i32>,
    b: MemoryDataset<i32>,
    c: MemoryDataset<i32>,
    d: MemoryDataset<i32>,
    df: PolarsDataset,
}

#[derive(Serialize, Deserialize)]
struct Parameters {
    initial_value: Param<i32>,
}

fn construct_pipe1(params: &Parameters, catalog: &Catalog) -> impl Steps {
    let pipe = (
        Node {
            func: |v| (v,),
            input: (&params.initial_value,),
            output: (&catalog.a,),
        },
        Pipeline {
            steps: (
                Node {
                    func: |v| (v + 2,),
                    input: (&catalog.a,),
                    output: (&catalog.b,),
                },
                Node {
                    func: |v| (v + 2,),
                    input: (&catalog.b,),
                    output: (&catalog.c,),
                },
            ),
            input: (&catalog.a,),
            output: (&catalog.c,),
        },
        Node {
            func: |v, a| (v + a + 2,),
            input: (&params.initial_value, &catalog.c),
            output: (&catalog.d,),
        },
        Node {
            func: |d| println!("{d}"),
            input: (&catalog.d,),
            output: (),
        },
    );
    pipe
}

fn construct_pipe2(params: &Parameters, catalog: &Catalog) -> impl Steps {
    let pipe = (
        Node {
            func: |v| (v,),
            input: (&params.initial_value,),
            output: (&catalog.a,),
        },
        Node {
            func: |v| (v * 3,),
            input: (&params.initial_value,),
            output: (&catalog.b,),
        },
        Node {
            func: |a, b| (a + b,),
            input: (&catalog.a, &catalog.b),
            output: (&catalog.c,),
        },
        Node {
            func: |c| println!("{c}"),
            input: (&catalog.c,),
            output: (),
        },
    );
    pipe
}

fn main() {
    let catalog = Catalog {
        a: MemoryDataset::new(),
        b: MemoryDataset::new(),
        c: MemoryDataset::new(),
        d: MemoryDataset::new(),
        df: PolarsDataset::new("test.parquet"),
    };
    let params = Parameters {
        initial_value: Param(2),
    };

    let params_yaml = serde_yaml::to_string(&params).unwrap();
    println!("{params_yaml}");
    let catalog_yaml = serde_yaml::to_string(&catalog).unwrap();
    println!("{catalog_yaml}");

    println!("--- Sequential Runner ---");
    let runner = SequentialRunner::new((LoggingHook,));
    let pipe = construct_pipe1(&params, &catalog);
    runner.run(&pipe);

    // Reset datasets for parallel run
    let catalog = Catalog {
        a: MemoryDataset::new(),
        b: MemoryDataset::new(),
        c: MemoryDataset::new(),
        d: MemoryDataset::new(),
        df: PolarsDataset::new("test.parquet"),
    };

    // Pipeline with independent nodes that can run in parallel:
    // param → a (node 1)
    // param → b (node 2) - independent of node 1
    // a, b → c (node 3) - waits for both
    // c → print (node 4)
    let pipe = construct_pipe2(&params, &catalog);

    println!("\n--- Parallel Runner ---");
    let runner = ParallelRunner::new((LoggingHook,));
    runner.run(&pipe);
}
