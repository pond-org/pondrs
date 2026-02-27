use std::collections::HashMap;

use polars::frame::DataFrame;
use pondrs::datasets::{
    Lazy, LazyPartitionedDataset, MemoryDataset, Param, PartitionedDataset, PolarsCsvDataset,
    PolarsParquetDataset,
};
use pondrs::error::PondError;
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

fn construct_pipe1(params: &Parameters, catalog: &Catalog) -> impl Steps<PondError> {
    let pipe = (
        Node {
            name: "node1",
            func: |v| (v,),
            input: (&params.initial_value,),
            output: (&catalog.a,),
        },
        Pipeline {
            name: "inner_pipeline",
            steps: (
                Node {
                    name: "node2",
                    func: |v| (v + 2,),
                    input: (&catalog.a,),
                    output: (&catalog.b,),
                },
                Node {
                    name: "node3",
                    func: |v| (v + 2,),
                    input: (&catalog.b,),
                    output: (&catalog.c,),
                },
            ),
            input: (&catalog.a,),
            output: (&catalog.c,),
        },
        Node {
            name: "node4",
            func: |v, a| (v + a + 2,),
            input: (&params.initial_value, &catalog.c),
            output: (&catalog.d,),
        },
        Node {
            name: "node5",
            func: |d| println!("{d}"),
            input: (&catalog.d,),
            output: (),
        },
    );
    pipe
}

fn construct_pipe2(params: &Parameters, catalog: &Catalog) -> impl Steps<PondError> {
    let pipe = (
        Node {
            name: "node1",
            func: |v| (v,),
            input: (&params.initial_value,),
            output: (&catalog.a,),
        },
        Node {
            name: "node2",
            func: |v| (v * 3,),
            input: (&params.initial_value,),
            output: (&catalog.b,),
        },
        Node {
            name: "node3",
            func: |a, b| (a + b,),
            input: (&catalog.a, &catalog.b),
            output: (&catalog.c,),
        },
        Node {
            name: "node4",
            func: |c| println!("{c}"),
            input: (&catalog.c,),
            output: (),
        },
    );
    pipe
}

#[derive(Serialize)]
struct IrisCatalog {
    input: LazyPartitionedDataset<PolarsParquetDataset>,
    output: PartitionedDataset<PolarsParquetDataset>,
    output_csv: LazyPartitionedDataset<PolarsCsvDataset>,
}

fn copy_iris(input: HashMap<String, Lazy<DataFrame>>) -> Result<(HashMap<String, DataFrame>,), PondError> {
    let mut output = HashMap::<String, DataFrame>::new();
    for (name, df) in input {
        println!("Read {name}!");
        output.insert(name, df.load()?);
    }
    Ok((output,))
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
            name: "node1",
            func: copy_iris,
            input: (&catalog.input,),
            output: (&catalog.output,),
        },
        Node {
            name: "node2",
            func: copy_iris_to_csv,
            input: (&catalog.output,),
            output: (&catalog.output_csv,),
        },
    );
    let params = ();
    let runner = SequentialRunner::new((LoggingHook,));
    runner.run::<PondError>(&pipe, &catalog, &params).unwrap();
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
    runner.run::<PondError>(&pipe, &catalog, &params).unwrap();

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
    runner.run::<PondError>(&pipe, &catalog, &params).unwrap();
}
