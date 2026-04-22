# Typed Data Catalog and Data Pipelines

Disclaimer: All of the ideas presented here are mine and the text was written without AI.
Based on these ideas, I have implemented the [pondrs library](https://github.com/pond-org/pondrs)
with AI assistance. If you do not want to read AI generated code,
please stop at reading this post. Regardless, I appreciate input on the ideas!

## Abstract

Data pipelines that separate IO from transformations make code easier to read and modify.
I was missing a simple typed version of this programming model. From prototype
implementations in several languages, I concluded that Rust can capture the pattern nicely.
Here I try to convey how to capture the model in Rust's trait system using a few
simple traits that work even on embedded (`no_std`) targets.

## Background

In my professional life I used to work on some python projects with elements of
data processing. Some were classical data pipelines and some were more complex, but all
shared the property that there was a set of inputs to the program, and it ran until
it produced a set of outputs. The team consolidated on using the [kedro](https://kedro.org/)
python library for many of these projects. Personally, I liked the structure it imposed
on the python code bases. At the highest level, it stores datasets in a "data catalog"
and then defines pipelines of nodes that map input datasets to output datasets.
In my perception, PRs to kedro code bases were easier to review, mostly due to three things:

* Firstly, the **graph structure** (and visualizations thereof) enables better code base understanding.
* Secondly, the **"functional purity"** of transformations helps when reasoning about impacts of changes.
* Thirdly, it comes with **standards** for how to configure parameters and paths etc., and how to run the programs.

> **EXAMPLE:** Kedro assumes a standard format for the project setup and pipeline
> definition. It then provides tools that allow you to run the pipeline in different ways,
> overload parameters, and launch interactive visualizations. We want to provide something
> similar for rust using a minimal interface.
> ```sh
> $ kedro run                           # run the default pipeline
> $ kedro run --params threshold=0.8    # override params from CLI
> $ kedro run --runner=SequentialRunner # run using sequential runner
> $ kedro run --pipeline=my_pipeline    # choose which pipeline to run
> $ kedro run --from-nodes="preprocess_data" --to-nodes="train_model"
> $ kedro viz run                       # interactive pipeline visualization
> ```

I felt like there was a broader programming paradigm hidden in all of this,
and that it wasn't yet fully formed. In particular, I felt like it was sorely lacking
typing and type completion. Thus began my years-long effort to uncover the
perfect implementation of these concepts. I'll spare you the details, but I've come
to believe that the paradigm (which I've come to call "Pipelines Over Nodes and Datasets")
can be perfectly realized in the Rust trait system.

Concretely, I'll show you how with a few simple traits, we can get typed datasets
and parameters, that can be strung together by standard rust functions (the nodes) into pipelines.
The pipeline structure can be validated prior to running, and be executed in different ways,
for example across different threads. This all builds on Rust's existing type system
with guarantees for memory and thread safety. The way this is built also means that
we can run the same pipelines on `no_std` targets, without heap allocation!

## The Dataset trait

If you are unfamiliar with Kedro, I recommend a
[recent blog post](https://kedro.org/blog/kedro-in-the-data-and-ai-landscape)
that touches on many of the concepts. 
We aim to separate loading and storing of data from the actual data transformation.
For this purpose, we define a `Dataset` trait, which provides methods for loading
and storing data. The types to be loaded and stored are defined by two different
associated types. This separation will be useful for read-only datasets like `Param`, shown below.

```rust
pub trait Dataset: serde::Serialize {
    type LoadItem;
    type SaveItem;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
}
```

As a side note, we need `serde::Serialize` as a supertrait since we need to be
able to walk the data structure tree using a custom serializer.

The vast majority of datasets will be stored using files, a database or
some web-based storage. Usually we define the parameters of the storage
as fields on the struct and just serialize them using the standard serde serializer.

For the remainder of the examples, we will use a custom error enum `CustomError` for the concrete error type.
We use the thiserror crate to convert low-level errors into a broader type for the whole pipeline.

> **EXAMPLE:** `PolarsParquetDataset` is a typical file backed dataset that has a single
> `path` parameter for the location of the file. 
> ```rust
> #[derive(Serialize, Deserialize)]
> pub struct PolarsParquetDataset {
>     pub path: String,
> }
> 
> impl Dataset for PolarsParquetDataset {
>     type LoadItem = DataFrame;
>     type SaveItem = DataFrame;
>     type Error = CustomError;
> 
>     fn load(&self) -> Result<Self::LoadItem, CustomError> {
>         let file = std::fs::File::open(&self.path)?;
>         let df = ParquetReader::new(file).finish()?;
>         Ok(df)
>     }
> 
>     fn save(&self, mut df: Self::SaveItem) -> Result<(), CustomError> {
>         let mut file = std::fs::File::create(&self.path)?;
>         ParquetWriter::new(&mut file).finish(&mut df)?;
>         Ok(())
>     }
> }
> ```

Parameters are important enough to also discuss here. They illustrate the usefulness
of having different load and save types, and might be the simplest instances of `Dataset`.

> **EXAMPLE:** In the case of parameters, we are only interested in reading their values:
> ```rust
> #[derive(Serialize, Deserialize)]
> pub struct Param<T: Clone>(pub T);
> 
> impl<T: Clone + Serialize> Dataset for Param<T> {
>     type LoadItem = T;
>     type SaveItem = (); // Param is never saved, only constructed!
>     type Error = Infallible;
> 
>     fn load(&self) -> Result<Self::LoadItem, Infallible> {
>         Ok(self.0.clone())
>     }
> 
>     fn save(&self, _output: Self::SaveItem) -> Result<(), Infallible> {
>         unreachable!("Param is read-only — save() should never be called")
>     }
> }
> ```

## Catalog and Params

The idea for getting a typed data catalog and parameters is to simply define them
using serde serializable structs over members implementing `Dataset`.
For datasets to be loadable from config files, they need to also implement `Deserialize`.

> **EXAMPLE:** Simply include the datasets you need in your data pipeline
> within one struct, and the parameters in another.
> ```rust
> #[derive(Serialize, Deserialize)]
> struct Catalog {
>     readings: PolarsParquetDataset,
>     summary: MemoryDataset<f64>,
>     report: JsonDataset,
> }
>
> #[derive(Serialize, Deserialize)]
> struct Params {
>     threshold: Param<f64>,
> }
> ```

We use serde to configure these e.g. from YAML files (or any other format serde supports).
> **EXAMPLE:** By convention we configure these using separate files.
> Note the `path` parameter here maps directly to the member in the `PolarsParquetDataset`
> above. `MemoryDataset`s such as `summary` have no parameters since they are set in the pipeline.
> Nevertheless, they need to be initialized using an empty `{}`.
> ```yaml
> # conf/base/catalog.yml
> readings:
>   path: data/readings.parquet
> summary: {}
> report:
>   path: data/report.json
> ```
> Parameters can be naturally configured using the struct member names:
> ```yaml
> # conf/base/parameters.yml
> threshold: 0.5
> ```
This setup gives us the standard configuration/parameters files of something like kedro,
with the added benefit that the values are all typed, and verified to be
well-formed at load time.

## The NodeInput & NodeOutput traits

A node is a transformation that takes an ordered set of input
datasets and produces an ordered set of outputs.
We represent these sets as tuples. The `NodeInput` trait is implemented for tuples of
dataset references, and its associated `Args` type is the corresponding tuple of loaded values,
i.e. the argument types of the node function.

```rust
// NodeInput should be implemented for tuples of datasets
pub trait NodeInput: Tuple {
    type Args: Tuple; // Tuple of return types of dataset load functions
    fn load_data(&self) -> Result<Self::Args, CustomError>; // Load all datasets
}

// Example implementation for 2-Tuples
// In the actual code, we use macros to implement arities 0-10
impl<T1: Dataset, T2: Dataset> NodeInput for (&T1, &T2)
where
    CustomError: From<T1::Error>,
    CustomError: From<T2::Error>,
{
    type Args = (T1::LoadItem, T2::LoadItem);
    fn load_data(&self) -> Result<Self::Args, CustomError> {
        Ok((self.0.load()?, self.1.load()?))
    }
}
```
The `NodeOutput` is defined similarly:
```rust
// NodeOutput should be implemented for tuples of datasets
pub trait NodeOutput: Tuple {
    type Output: Tuple; // Tuple of function argument types of dataset save functions 
    fn save_data(&self, output: Self::Output) -> Result<(), CustomError>; // Save all datasets
}
```
We omit the example implementation in this case, since it is similar to `NodeInput`.
It calls `Dataset::save` for all elements of the argument tuple (e.g. `self.1.save(output.1)?`).

## The Node struct

The `Node` struct representing the actual transformation
needs to accommodate functions that return either
a plain tuple or a `Result`. For this purpose, we define
a trait that allows being generic over output with or without errors.
```rust
pub trait CompatibleOutput<O: Tuple> {}

impl<O: Tuple> CompatibleOutput<O> for O {}
impl<O: Tuple, E> CompatibleOutput<O> for Result<O, E> {}
```
For the purposes of this post, we will use the unstable `Fn` and `Tuple` traits since people
might already be familiar with those. In the actual code we use
custom `StableTuple` and `StableFn` stable-Rust equivalents that
will be removed once the nightly features stabilize.

Similar to kedro, each `Node` instance is identified by a name, thereby allowing a clear
description, as well as reuse of the same function multiple times within the pipeline.
Using the `NodeInput`, `NodeOutput` and `CompatibleOutput`, we can define
the struct as follows:
```rust
pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: Fn<Input::Args>,
    F::Output: CompatibleOutput<Output::Output>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}
```
One huge advantage of this formulation is that we get a type error directly at the
`Node` definition if the function does not match the types provided by
the input or output datasets.

With this machinery, it is straightforward to load the data, run the node function,
and save it back to the data catalog:
```rust
let args = node.input.load_data()?;
let result = Fn::call(&node.func, args); // (StableFn::call in the actual crate)
node.output.save_data(result)?; // In real code a little more complex bc of CompatibleOutput
```
This is basically the way in which nodes get executed in the pipelines.

## Steps trait and Pipelines

Nodes are not very useful by themselves. Now we'll see how we can string them
together with datasets in between in order to create data flow.
The `Steps` trait is implemented for tuples (or general sequences) of
`RunnableStep`s. For the purpose of this overview, you can just think of them as nodes.
The abstraction exists because pondrs also has a `Pipeline` type
(a group of `Steps` with declared inputs/outputs) that can appear wherever a `Node` can.
```rust
pub trait Steps<E> {
    /// Iterate over each executable step.
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>));
}
```
Notice that both `Steps` and `RunnableStep` are generic over the error type that
is used in the pipeline. Errors returned by the nodes or datasets need to be
convertible to this type in order for the program to type check.

The `for_each_item` method can be invoked by a runner to iterate over the steps —
the sequential runner executes them in the declared order,
while the parallel runner uses the iteration to
build a dependency graph and run independent nodes concurrently.

By convention, and for the purpose of having a standard interface, we typically define
our top-level pipelines as functions mapping catalog and parameter structs to an object implementing `Steps`:
> **EXAMPLE:** The top-level pipeline is defined by a function returning a steps object.
> Notice that all inputs and outputs are defined by references to struct members. This
> allows us to verify if they are pointing to the same object.
> ```rust
> fn my_pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<CustomError> + 'a {
>     (
>         Node {
>             name: "summarize",
>             func: |df: DataFrame| -> Result<(f64,), CustomError> {
>                 let mean = df.column("value")?.f64()?.mean()?;
>                 Ok((mean,))
>             },
>             input: (&cat.readings,),
>             output: (&cat.summary,),
>         },
>         Node {
>             name: "report",
>             func: |mean: f64, threshold: f64| {
>                 (json!({ "mean": mean, "passed": mean >= threshold }),)
>             },
>             input: (&cat.summary, &params.threshold),
>             output: (&cat.report,),
>         },
>     )
> }
> ```
Because we use `impl Trait` here, the full pipeline object is stack-allocated —
in keeping with our `no_std` aspiration.

Besides running the pipeline, it is also important to validate that it is well formed.
For example, we need to ensure that inputs to later nodes are produced by previous nodes,
and that a catalog entry is not produced by multiple nodes. This is why we are taking
the catalog and parameter struct by reference. Since all nodes are pointing to the same
objects in memory, we simply take the pointer of the input and output datasets of different
nodes in order to know if they are referring to the same thing:

```rust
pub fn ptr_to_id<T: ?Sized>(r: &T) -> usize {
    r as *const T as *const () as usize
}
```
Note that casting a pointer to `usize` is safe Rust — no `unsafe` blocks are needed anywhere in this machinery.
We won't go into depth on the verification logic here since it requires quite a lot
of exposition. The general principle entails looking at the pointer ids.
To ensure a valid pipeline, nodes producing a dataset with one id should come before
nodes consuming a dataset with the same id.
If node A's output is `&cat.summary` and node B's input is `&cat.summary`,
`ptr_to_id` will return the same value and we know B depends on A.
The `check` subcommand (shown below) runs this validation without executing the pipeline.

## Wrap it all up into the App builder

Now we'll focus on how to package a pipeline into a standard executable that can be
invoked in the standard way. Since the interface needs to be configurable we
use a builder pattern to construct the `App` object. The full `App` definition is
beyond this post's scope — see the [App docs](https://docs.rs/pondrs/latest/pondrs/app/struct.App.html).
The `App` struct takes a pipeline function and packages it up into a program
with a rich command line interface that supports several subcommands and parameters:
> **EXAMPLE:**
> ```rust
> fn main() -> Result<(), CustomError> {
>     App::from_args(std::env::args_os())?
>         .dispatch(my_pipeline)
> }
> ```
This gives us a command line interface similar to kedro, but contained within the program executable.
If you don't need CLI arguments, you can even compile it for `no_std` targets without heap allocation.
> **EXAMPLE:** Run the executable with the different subcommands. Note that while
> this overview has mostly covered the "run" functionality, the same trait hierarchy
> also allows validating the pipeline structure, and producing graphs with metadata
> about nodes and datasets that can be used e.g. for visualization.
> ```sh
> $ my_app run                          # run the whole pipeline
> $ my_app run --params threshold=0.8   # override params from CLI
> $ my_app run --runner sequential      # run using the sequential runner
> $ my_app run --from-nodes summarize --to-nodes report
> $ my_app check                        # validate pipeline DAG
> $ my_app viz                          # interactive pipeline visualization
> ```

## Conclusion

And that's it! With a relatively straightforward trait system, we have managed to replicate
most of the structure and interface of kedro. In addition to that, we get all the benefits
of Rust, with strict typing, fearless concurrency and efficient execution!

This is my current best approximation of this programming pattern in Rust.
It is still a work in progress and things might evolve as I learn more about the
language. Also, as the language evolves, we might be able to remove some of the current
warts, such as the stopgap solutions for the `Fn` and `Tuple` traits, as well as
macro-impls for a fixed set of tuple arities.
