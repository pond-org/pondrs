//! Datasets module for yggdrasil.

// use std::fs;
//use polars::prelude::*;
use polars::prelude::{CsvReadOptions, CsvWriter, DataFrame, SerReader, SerWriter};
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::marker::Tuple;
use std::sync::{Arc, Mutex};

use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

pub trait Dataset {
    type LoadItem;
    type SaveItem;

    fn load(&self) -> Option<Self::LoadItem>;
    fn save(&self, output: Self::SaveItem);
}

#[derive(Serialize, Deserialize)]
pub struct YamlDataset {
    path: String,
}

impl Dataset for YamlDataset {
    type LoadItem = Yaml;
    type SaveItem = Yaml;

    fn save(&self, yaml: Self::SaveItem) {
        let mut out_str = String::new();
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&yaml).unwrap();
        std::fs::write(&self.path, &out_str).unwrap();
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let contents = std::fs::read_to_string(&self.path).unwrap();
        let docs = YamlLoader::load_from_str(&contents).unwrap();
        Some(docs[0].clone())
    }
}

#[derive(Serialize, Deserialize)]
pub struct PolarsDataset {
    path: String,
}

impl Dataset for PolarsDataset {
    type LoadItem = DataFrame;
    type SaveItem = DataFrame;

    fn save(&self, mut df: Self::SaveItem) {
        let mut file = std::fs::File::create(&self.path).unwrap();
        CsvWriter::new(&mut file).finish(&mut df).unwrap();
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some(self.path.clone().into()))
            .unwrap()
            .finish()
            .unwrap();
        Some(df)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Param<T: Clone>(T);
impl<T: Clone> Dataset for Param<T> {
    type LoadItem = T;
    type SaveItem = ();
    fn load(&self) -> Option<Self::LoadItem> {
        Some(self.0.clone())
    }
    fn save(&self, _output: Self::SaveItem) {}
}

#[derive(Serialize, Deserialize)]
pub struct MemoryDataset<T: Clone> {
    #[serde(skip_serializing, skip_deserializing)]
    value: Arc<Mutex<Option<T>>>,
}

impl<T: Clone> MemoryDataset<T> {
    fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T: Copy> Dataset for MemoryDataset<T> {
    type LoadItem = T;
    type SaveItem = T;
    fn load(&self) -> Option<Self::LoadItem> {
        *self.value.lock().unwrap()
    }
    fn save(&self, output: Self::SaveItem) {
        let mut value = self.value.lock().unwrap();
        *value = Some(output);
    }
}

pub trait NodeInput: Tuple {
    type Args: Tuple;
    fn load_data(&self) -> Self::Args;
}

impl NodeInput for () {
    type Args = ();
    fn load_data(&self) -> Self::Args {
        ()
    }
}

impl<T: Dataset> NodeInput for (&T,) {
    type Args = (T::LoadItem,);
    fn load_data(&self) -> Self::Args {
        (self.0.load().unwrap(),)
    }
}

impl<T1: Dataset, T2: Dataset> NodeInput for (&T1, &T2) {
    type Args = (T1::LoadItem, T2::LoadItem);
    fn load_data(&self) -> Self::Args {
        (self.0.load().unwrap(), self.1.load().unwrap())
    }
}

pub trait NodeOutput: Tuple {
    type Output: Tuple;
    fn save_data(&self, output: Self::Output);
}

impl NodeOutput for () {
    type Output = ();
    fn save_data(&self, _output: Self::Output) {}
}

impl<T: Dataset> NodeOutput for (&T,) {
    type Output = (T::SaveItem,);
    fn save_data(&self, output: Self::Output) {
        self.0.save(output.0);
    }
}

impl<T1: Dataset, T2: Dataset> NodeOutput for (&T1, &T2) {
    type Output = (T1::SaveItem, T2::SaveItem);
    fn save_data(&self, output: Self::Output) {
        self.0.save(output.0);
        self.1.save(output.1);
    }
}

trait NodeSig {
    // type Input: NodeInput;
    // type Output: NodeOutput;
    fn call(&self);
    fn get_name(&self) -> &'static str;
}

struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: Fn<Input::Args, Output = Output::Output>,
{
    func: F,
    input: Input,
    output: Output,
}

impl<F, Input: NodeInput, Output: NodeOutput> NodeSig for Node<F, Input, Output>
where
    F: Fn<Input::Args, Output = Output::Output>,
{
    // type Input = Input;
    // type Output = Output;
    fn call(&self) {
        let args = self.input.load_data();
        let outputs = Fn::call(&self.func, args);
        self.output.save_data(outputs);
    }
    fn get_name(&self) -> &'static str {
        std::any::type_name::<F>()
    }
}

trait Pipeline: Tuple {
    fn for_each(&self, f: impl Fn(&dyn NodeSig));
}

impl<N1: NodeSig, N2: NodeSig, N3: NodeSig> Pipeline for (N1, N2, N3) {
    fn for_each(&self, f: impl Fn(&dyn NodeSig)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
    }
}

#[derive(Serialize, Deserialize)]
struct Catalog {
    a: MemoryDataset<i32>,
    b: MemoryDataset<i32>,
    df: PolarsDataset,
}

#[derive(Serialize, Deserialize)]
struct Parameters {
    initial_value: Param<i32>,
}

trait Hook {
    fn before_node_run(&mut self, _n: &impl NodeSig) {}
    fn after_node_run(&mut self, _n: &impl NodeSig) {}
}

pub struct LoggingHook;

impl Hook for LoggingHook {
    fn before_node_run(&mut self, n: &impl NodeSig) {
        let name = n.get_name();
        println!("Starting node {name}")
    }
    fn after_node_run(&mut self, n: &impl NodeSig) {
        let name = n.get_name();
        println!("Completed node {name}")
    }
}

pub fn test() {
    let catalog = Catalog {
        a: MemoryDataset::new(),
        b: MemoryDataset::new(),
        df: PolarsDataset {
            path: "test.parquet".to_string(),
        },
    };
    let params = Parameters {
        initial_value: Param(2),
    };
    let params_yaml = serde_yaml::to_string(&params).unwrap();
    println!("{params_yaml}");
    let catalog_yaml = serde_yaml::to_string(&catalog).unwrap();
    println!("{catalog_yaml}");
    let pipe = (
        Node {
            func: |v| (v,),
            input: (&params.initial_value,),
            output: (&catalog.a,),
        },
        Node {
            func: |v, a| (v + a + 2,),
            input: (&params.initial_value, &catalog.a),
            output: (&catalog.b,),
        },
        Node {
            func: |b| println!("{b}"),
            input: (&catalog.b,),
            output: (),
        },
    );
    pipe.for_each(|n| n.call());
}
