//! Application framework for building CLI executables from pipeline components.

pub mod cli;
pub mod config;

use std::prelude::v1::*;
use std::process;

use clap::Parser;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::core::Steps;
use crate::error::PondError;
use crate::graph::build_pipeline_graph;
use crate::hooks::Hooks;
use crate::runners::{ParallelRunner, Runners, SequentialRunner};

use cli::{CliArgs, Command};
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
    /// Users write: `type Pipeline<'a> = impl Steps<Self::Error>;`
    type Pipeline<'a>: Steps<Self::Error>
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

    /// Provide the available runners as a tuple. The default runner is `"sequential"`.
    /// Override to customize which runners are available.
    fn runners() -> impl Runners {
        (SequentialRunner, ParallelRunner)
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
        Self::main_from(std::env::args_os());
    }

    /// Like `main()`, but parses CLI args from `iter` instead of `std::env::args`.
    /// Useful for examples and integration tests that need to supply paths at runtime.
    ///
    /// ```ignore
    /// SalesApp::main_from(["sales-app", "--catalog-path", "cat.yml",
    ///                      "--params-path", "params.yml", "run"]);
    /// ```
    fn main_from<I, T>(iter: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let args = CliArgs::parse_from(iter);

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
                let runners = Self::runners();

                let name = runner.as_deref().unwrap_or("sequential");

                match runners.run_by_name(name, &pipeline, &catalog, &params, &hooks) {
                    Some(result) => result,
                    None => {
                        eprint!("Error: unknown runner '{name}'. Available runners: ");
                        let mut first = true;
                        runners.for_each_name(&mut |n| {
                            if !first { eprint!(", "); }
                            eprint!("{n}");
                            first = false;
                        });
                        eprintln!();
                        process::exit(1);
                    }
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
