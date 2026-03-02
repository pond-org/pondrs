//! Application framework for building CLI executables from pipeline components.

pub mod cli;
pub mod config;

use std::prelude::v1::*;
use std::process;

use clap::Parser;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::core::{StepInfo, Steps};
use crate::error::PondError;
use crate::graph::build_pipeline_graph;
use crate::hooks::Hooks;
use crate::runners::{NoRunner, ParallelRunner, Runner, SequentialRunner};

use cli::{CliArgs, Command, RunnerChoice};
use config::{apply_overrides, deserialize_config, load_yaml};

/// Load a YAML config file, apply overrides, and deserialize into concrete type.
fn load_config<T: DeserializeOwned, E: From<PondError>>(
    path: &str,
    overrides: &[String],
) -> Result<T, E> {
    let mut value = load_yaml(path)?;
    if !overrides.is_empty() {
        apply_overrides(&mut value, overrides);
    }
    Ok(deserialize_config(value)?)
}

/// Trait for building a full CLI executable from pipeline components.
///
/// Implement this trait on a unit struct, then call `MyApp::main()` from your
/// binary's `main()` function. The framework handles CLI parsing, YAML config
/// loading, param overrides, and subcommand dispatch.
pub trait PondApp {
    /// The catalog struct containing all datasets. Must be deserializable from YAML
    /// and serializable (for the catalog indexer).
    type Catalog: Serialize + DeserializeOwned;

    /// The params struct containing all `Param<T>` fields. Must be deserializable
    /// from YAML and serializable.
    type Params: Serialize + DeserializeOwned;

    /// The pipeline error type.
    type Error: From<PondError> + Send + Sync + core::fmt::Display + core::fmt::Debug + 'static;

    /// The pipeline type, parameterized by the borrow lifetime into catalog/params.
    /// Users write: `type Pipeline<'a> = impl Steps<Self::Error> + StepInfo;`
    type Pipeline<'a>: Steps<Self::Error> + StepInfo
    where
        Self::Catalog: 'a,
        Self::Params: 'a;

    /// Build the pipeline from catalog and params references.
    fn pipeline<'a>(
        catalog: &'a Self::Catalog,
        params: &'a Self::Params,
    ) -> Self::Pipeline<'a>;

    /// Provide hooks for pipeline execution.
    /// Return `()` for no hooks.
    fn hooks() -> impl Hooks;

    /// Built-in sequential runner. Override to return `None::<NoRunner>` to disable.
    fn sequential_runner() -> Option<impl Runner> {
        Some(SequentialRunner)
    }

    /// Built-in parallel runner. Override to return `None::<NoRunner>` to disable.
    fn parallel_runner() -> Option<impl Runner> {
        Some(ParallelRunner)
    }

    /// Optional custom runner. When `Some`, this becomes the default runner.
    fn custom_runner() -> Option<impl Runner> {
        None::<NoRunner>
    }

    /// Path to the catalog YAML config file.
    fn catalog_path() -> &'static str {
        "conf/base/catalog.yml"
    }

    /// Path to the parameters YAML config file.
    fn params_path() -> &'static str {
        "conf/base/parameters.yml"
    }

    /// Full CLI entrypoint. Parses args, loads config, dispatches subcommand.
    fn main() {
        let args = CliArgs::parse();

        let catalog_path = args.catalog_path.as_deref().unwrap_or(Self::catalog_path());
        let params_path = args.params_path.as_deref().unwrap_or(Self::params_path());

        let result: Result<(), Self::Error> = match args.command {
            Command::Run {
                runner,
                param_overrides,
                catalog_overrides,
            } => {
                let catalog: Self::Catalog = match load_config::<Self::Catalog, Self::Error>(catalog_path, &catalog_overrides) {
                    Ok(c) => c,
                    Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                };
                let params: Self::Params = match load_config::<Self::Params, Self::Error>(params_path, &param_overrides) {
                    Ok(p) => p,
                    Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                };
                let pipeline = Self::pipeline(&catalog, &params);
                let hooks = Self::hooks();

                let has_custom = Self::custom_runner().is_some();
                let choice = runner.unwrap_or(if has_custom {
                    RunnerChoice::Custom
                } else {
                    RunnerChoice::Sequential
                });

                match choice {
                    RunnerChoice::Sequential => match Self::sequential_runner() {
                        Some(r) => r.run(&pipeline, &catalog, &params, &hooks),
                        None => { eprintln!("Error: sequential runner is disabled."); process::exit(1); }
                    },
                    RunnerChoice::Parallel => match Self::parallel_runner() {
                        Some(r) => r.run(&pipeline, &catalog, &params, &hooks),
                        None => { eprintln!("Error: parallel runner is disabled."); process::exit(1); }
                    },
                    RunnerChoice::Custom => match Self::custom_runner() {
                        Some(r) => r.run(&pipeline, &catalog, &params, &hooks),
                        None => { eprintln!("Error: no custom runner configured."); process::exit(1); }
                    },
                }
            }
            Command::Check => {
                let catalog: Self::Catalog = match load_config::<Self::Catalog, Self::Error>(catalog_path, &[]) {
                    Ok(c) => c,
                    Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                };
                let params: Self::Params = match load_config::<Self::Params, Self::Error>(params_path, &[]) {
                    Ok(p) => p,
                    Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                };
                let pipeline = Self::pipeline(&catalog, &params);

                let graph = build_pipeline_graph(&pipeline, &catalog, &params);
                match graph.check() {
                    Ok(()) => {
                        let num_nodes = graph.node_indices.len();
                        let num_datasets = graph.dataset_names.len();
                        println!("Pipeline is valid: {num_nodes} nodes, {num_datasets} datasets.");
                        Ok(())
                    }
                    Err(errors) => {
                        eprintln!("Pipeline validation failed:");
                        for err in &errors {
                            eprintln!("  - {err}");
                        }
                        process::exit(1);
                    }
                }
            }
            Command::Viz { .. } => {
                eprintln!("Error: viz subcommand is not yet implemented.");
                process::exit(1);
            }
        };

        if let Err(e) = result {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}
