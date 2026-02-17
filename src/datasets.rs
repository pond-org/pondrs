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

trait PipelineItem {
    // type Input: NodeInput;
    // type Output: NodeOutput;
    fn call(&self);
    fn get_name(&self) -> &'static str;
    fn is_leaf(&self) -> bool;
    fn for_each_child(&self, f: &mut dyn FnMut(&dyn PipelineItem));
}

struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: Fn<Input::Args, Output = Output::Output>,
{
    func: F,
    input: Input,
    output: Output,
}

impl<F, Input: NodeInput, Output: NodeOutput> PipelineItem for Node<F, Input, Output>
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

    fn is_leaf(&self) -> bool {
        true
    }
    fn for_each_child(&self, _f: &mut dyn FnMut(&dyn PipelineItem)) {
        // No children, do nothing
    }
}

trait Steps: Tuple {
    fn for_each_item(&self, f: &mut dyn FnMut(&dyn PipelineItem));
}

struct Pipeline<S: Steps, Input: NodeInput, Output: NodeOutput> {
    steps: S,
    input: Input,
    output: Output,
}

impl<S: Steps, Input: NodeInput, Output: NodeOutput> PipelineItem for Pipeline<S, Input, Output> {
    fn call(&self) {}
    fn get_name(&self) -> &'static str {
        "pipeline"
    }
    fn is_leaf(&self) -> bool {
        false
    }
    fn for_each_child(&self, f: &mut dyn FnMut(&dyn PipelineItem)) {
        self.steps.for_each_item(f);
    }
}

impl<N1: PipelineItem> Steps for (N1,) {
    fn for_each_item(&self, f: &mut dyn FnMut(&dyn PipelineItem)) {
        f(&self.0);
    }
}

impl<N1: PipelineItem, N2: PipelineItem> Steps for (N1, N2) {
    fn for_each_item(&self, f: &mut dyn FnMut(&dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
    }
}

impl<N1: PipelineItem, N2: PipelineItem, N3: PipelineItem> Steps for (N1, N2, N3) {
    fn for_each_item(&self, f: &mut dyn FnMut(&dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
    }
}

impl<N1: PipelineItem, N2: PipelineItem, N3: PipelineItem, N4: PipelineItem> Steps
    for (N1, N2, N3, N4)
{
    fn for_each_item(&self, f: &mut dyn FnMut(&dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
    }
}

impl<N1: PipelineItem, N2: PipelineItem, N3: PipelineItem, N4: PipelineItem, N5: PipelineItem> Steps
    for (N1, N2, N3, N4, N5)
{
    fn for_each_item(&self, f: &mut dyn FnMut(&dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
        f(&self.4);
    }
}

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

trait Hook {
    fn before_node_run(&mut self, _n: &dyn PipelineItem) {}
    fn after_node_run(&mut self, _n: &dyn PipelineItem) {}
}

trait Hooks {
    fn for_each_hook(&mut self, f: &mut dyn FnMut(&mut dyn Hook));
}

impl Hooks for () {
    fn for_each_hook(&mut self, _f: &mut dyn FnMut(&mut dyn Hook)) {}
}

impl<H: Hook> Hooks for (H,) {
    fn for_each_hook(&mut self, f: &mut dyn FnMut(&mut dyn Hook)) {
        f(&mut self.0);
    }
}

impl<H1: Hook, H2: Hook> Hooks for (H1, H2) {
    fn for_each_hook(&mut self, f: &mut dyn FnMut(&mut dyn Hook)) {
        f(&mut self.0);
        f(&mut self.1);
    }
}

impl<H1: Hook, H2: Hook, H3: Hook> Hooks for (H1, H2, H3) {
    fn for_each_hook(&mut self, f: &mut dyn FnMut(&mut dyn Hook)) {
        f(&mut self.0);
        f(&mut self.1);
        f(&mut self.2);
    }
}

pub struct LoggingHook;

impl Hook for LoggingHook {
    fn before_node_run(&mut self, n: &dyn PipelineItem) {
        let name = n.get_name();
        println!("Starting node {name}")
    }
    fn after_node_run(&mut self, n: &dyn PipelineItem) {
        let name = n.get_name();
        println!("Completed node {name}")
    }
}

trait Runner {
    fn run(&mut self, pipe: &impl Steps);
}

struct SequentialRunner<H: Hooks> {
    hooks: H,
}

impl<H: Hooks> SequentialRunner<H> {
    fn run_item(&mut self, item: &dyn PipelineItem) {
        if item.is_leaf() {
            self.hooks.for_each_hook(&mut |h| h.before_node_run(item));
            item.call();
            self.hooks.for_each_hook(&mut |h| h.after_node_run(item));
        } else {
            // Optionally: hooks.for_each_hook(&mut |h| h.before_pipeline_run(item));
            item.for_each_child(&mut |child| {
                self.run_item(child);
            });
            // Optionally: hooks.for_each_hook(&mut |h| h.after_pipeline_run(item));
        }
    }
}

impl<H: Hooks> Runner for SequentialRunner<H> {
    fn run(&mut self, pipe: &impl Steps) {
        pipe.for_each_item(&mut |item| {
            self.run_item(item);
        });
    }
}

pub fn test() {
    let catalog = Catalog {
        a: MemoryDataset::new(),
        b: MemoryDataset::new(),
        c: MemoryDataset::new(),
        d: MemoryDataset::new(),
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
    // pipe.for_each_item(&mut |n| n.call());
    let mut runner = SequentialRunner {
        hooks: (LoggingHook,),
    };
    runner.run(&pipe);
}
