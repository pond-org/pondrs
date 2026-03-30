# Pondrs and Kedro

[Pondrs](https://github.com/pond-org/pondrs) is short for “Pipelines of Nodes & Datasets”.
As it says on the can, it is a pipeline/DAG/ETL processing framework for rust.
It was born out of plenty of experience and admiration for the [kedro](https://kedro.org/) python framework.
Pondrs borrows most of its user-facing concepts, and attempts to take them further with rust features like:

* Strong typing across pipeline and data catalog  
* Fearless concurrency across nodes/datasets/runners  
* Zero-cost abstractions, with ability to run pipelines with no\_std (heapless)  
* Validation of parameters using types

We think we have managed to keep things highly ergonomic, while upholding the simplicity and explicitness of kedro, including features like:

* Pipelines and nodes, with explicit inputs and outputs (which are typed\!)  
* Parameters which are set in config files and overloaded from command line  
* Data catalog containing datasets, configured in config files  
* Runners, sequential and parallel available with implementable trait  
* Hooks, with implementable trait  
* Visualization, which shows info on nodes and datasets, with previews and run diagnostics

Frameworks like kedro and pondrs are useful across a broad set of problems, not limited to simple data transformation.
Having a graph that illustrates high-level program logic eases codebase understanding.
And reviewing PRs that only affect pure functions carries less of a mental burden because of the lack of global or object state across node boundaries.
This is true regardless if the code was authored by a team mate or by AI\!

Speaking of AI: pondrs was written with the help of an AI agent. All design and architecture work was carried out by a human and all changes have been thoroughly reviewed.

The following resources are available if you wanna know more:

* [Github repo](https://github.com/pond-org/pondrs) (the code is Apache 2.0)  
* [Book](https://pond-org.github.io/pondrs) with examples  
* [Crate](https://crates.io/crates/pondrs) (Rust package)  
* [Docs](https://docs.rs/pondrs/latest/pondrs) (the docstrings are a work in progress)  
* [Example repo](https://github.com/pond-org/pondrs-examples) (to take and expand into your own pondrs application)

Pondrs is currently in active development, expect it to take a while to hit a stable release. Happy hacking!
