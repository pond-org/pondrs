use std::collections::HashMap;

use polars::frame::DataFrame;
use pondrs::datasets::{
    Lazy, LazyPartitionedDataset, MemoryDataset, Param, PartitionedDataset, PolarsCsvDataset,
    PolarsParquetDataset,
};
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
    df: PolarsParquetDataset,
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

struct IrisCatalog {
    input: LazyPartitionedDataset<PolarsParquetDataset>,
    output: PartitionedDataset<PolarsParquetDataset>,
    output_csv: LazyPartitionedDataset<PolarsCsvDataset>,
}

fn copy_iris(input: HashMap<String, Lazy<DataFrame>>) -> (HashMap<String, DataFrame>,) {
    let mut output = HashMap::<String, DataFrame>::new();
    for (name, df) in input {
        println!("Read {name}!");
        output.insert(name, df.load().unwrap());
    }
    (output,)
}

fn copy_iris_to_csv(input: HashMap<String, DataFrame>) -> (HashMap<String, DataFrame>,) {
    let mut output = HashMap::<String, DataFrame>::new();
    for (name, df) in input {
        println!("Read {name}!");
        output.insert(name, df);
    }
    (output,)
}

fn iris_test() {
    let catalog = IrisCatalog {
        input: LazyPartitionedDataset::<PolarsParquetDataset> {
            path: "iris".to_string(),
            ext: "parquet",
            dataset: PolarsParquetDataset {
                path: String::new(),
            },
        },
        output: PartitionedDataset::<PolarsParquetDataset> {
            path: "iris_copy".to_string(),
            ext: "parquet",
            dataset: PolarsParquetDataset {
                path: String::new(),
            },
        },
        output_csv: LazyPartitionedDataset::<PolarsCsvDataset> {
            path: "iris_csv".to_string(),
            ext: "csv",
            dataset: PolarsCsvDataset {
                path: String::new(),
            },
        },
    };
    let pipe = (
        Node {
            func: copy_iris,
            input: (&catalog.input,),
            output: (&catalog.output,),
        },
        Node {
            func: copy_iris_to_csv,
            input: (&catalog.output,),
            output: (&catalog.output_csv,),
        },
    );
    let runner = SequentialRunner::new((LoggingHook,));
    runner.run(&pipe);
}

fn main() {
    iris_test();
    let catalog = Catalog {
        a: MemoryDataset::new(),
        b: MemoryDataset::new(),
        c: MemoryDataset::new(),
        d: MemoryDataset::new(),
        df: PolarsParquetDataset::new("test.parquet"),
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
        df: PolarsParquetDataset::new("test.parquet"),
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
