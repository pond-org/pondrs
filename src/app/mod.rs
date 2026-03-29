//! Application framework for building pipeline executables.
//!
//! The [`App`] struct bundles catalog, params, hooks, and runners together
//! and provides methods for pipeline execution and CLI dispatch.

#[cfg(feature = "std")]
pub mod cli;
#[cfg(feature = "std")]
pub mod config;

#[cfg(feature = "std")]
use std::prelude::v1::*;

use serde::Serialize;

use crate::error::PondError;
use crate::hooks::Hooks;
use crate::pipeline::{PipelineFn, StepInfo};
use crate::runners::{Runners, SequentialRunner};

// --- Command enum (no_std core) ---

/// Subcommand for pipeline dispatch.
pub enum Command {
    /// Execute the pipeline.
    Run,
    /// Validate pipeline structure.
    Check,
    /// Build and serve pipeline visualization.
    #[cfg(feature = "std")]
    Viz {
        port: u16,
        output: Option<std::string::String>,
        export: Option<std::string::String>,
    },
}

// --- Default runners (conditional on std for ParallelRunner) ---

#[cfg(feature = "std")]
type DefaultRunners = (SequentialRunner, crate::runners::ParallelRunner);
#[cfg(not(feature = "std"))]
type DefaultRunners = (SequentialRunner,);

// --- App struct ---

/// Pipeline application with catalog, params, hooks, and runners.
///
/// # Construction
///
/// - [`App::new`] — provide catalog and params directly (no_std + std)
/// - [`App::from_cli`] — load from YAML via parsed [`CliArgs`](cli::CliArgs) (std only)
/// - [`App::from_args`] — parse CLI args and load from YAML (std only)
///
/// # Execution
///
/// - [`App::execute`] — run the pipeline directly
/// - [`App::dispatch`] — dispatch based on stored [`Command`]
pub struct App<C, P, H = (), R = DefaultRunners> {
    catalog: C,
    params: P,
    hooks: H,
    runners: R,
    command: Command,
    #[cfg(feature = "std")]
    runner_name: Option<std::string::String>,
    #[cfg(feature = "std")]
    node_filter: Option<crate::pipeline::NodeFilter>,
    #[cfg(feature = "std")]
    program_name: std::string::String,
}

// --- Core constructors (no_std) ---

impl<C, P> App<C, P, (), DefaultRunners> {
    /// Create an App with provided catalog and params.
    ///
    /// Uses default hooks (none) and default runners. Command defaults to `Run`.
    pub fn new(catalog: C, params: P) -> Self {
        App {
            catalog,
            params,
            hooks: (),
            runners: DefaultRunners::default(),
            command: Command::Run,
            #[cfg(feature = "std")]
            runner_name: None,
            #[cfg(feature = "std")]
            node_filter: None,
            #[cfg(feature = "std")]
            program_name: std::string::String::new(),
        }
    }
}

// --- Builder methods (no_std) ---

impl<C, P, H, R> App<C, P, H, R> {
    /// Replace hooks, returning a new App with different hook type.
    pub fn with_hooks<H2: Hooks>(self, hooks: H2) -> App<C, P, H2, R> {
        App {
            catalog: self.catalog,
            params: self.params,
            hooks,
            runners: self.runners,
            command: self.command,
            #[cfg(feature = "std")]
            runner_name: self.runner_name,
            #[cfg(feature = "std")]
            node_filter: self.node_filter,
            #[cfg(feature = "std")]
            program_name: self.program_name,
        }
    }

    /// Replace runners, returning a new App with different runner type.
    pub fn with_runners<R2: Runners>(self, runners: R2) -> App<C, P, H, R2> {
        App {
            catalog: self.catalog,
            params: self.params,
            hooks: self.hooks,
            runners,
            command: self.command,
            #[cfg(feature = "std")]
            runner_name: self.runner_name,
            #[cfg(feature = "std")]
            node_filter: self.node_filter,
            #[cfg(feature = "std")]
            program_name: self.program_name,
        }
    }

    /// Borrow the catalog.
    pub fn catalog(&self) -> &C {
        &self.catalog
    }

    /// Borrow the params.
    pub fn params(&self) -> &P {
        &self.params
    }

    /// Set the command for dispatch.
    pub fn with_command(mut self, command: Command) -> Self {
        self.command = command;
        self
    }

    /// Borrow the current command.
    pub fn command(&self) -> &Command {
        &self.command
    }
}

// --- Execution methods (no_std) ---

impl<C: Serialize, P: Serialize, H: Hooks, R: Runners> App<C, P, H, R> {
    /// Execute the pipeline directly.
    ///
    /// Uses the runner selected by CLI (`--runner`), defaulting to `"sequential"`.
    pub fn execute<'a, E, F>(&'a self, f: F) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
        F: PipelineFn<'a, C, P, E>,
    {
        let pipeline = f.call(&self.catalog, &self.params);

        #[cfg(feature = "std")]
        let name = self.runner_name.as_deref().unwrap_or(self.runners.first_name());
        #[cfg(not(feature = "std"))]
        let name = self.runners.first_name();

        #[cfg(feature = "std")]
        if let Some(ref filter) = self.node_filter {
            let filtered = crate::pipeline::filter_steps(&pipeline, &self.catalog, &self.params, filter)
                .map_err(E::from)?;
            return match self.runners.run_by_name(name, &filtered, &self.catalog, &self.params, &self.hooks) {
                Some(result) => result,
                None => Err(PondError::RunnerNotFound.into()),
            };
        }

        match self.runners.run_by_name(name, &pipeline, &self.catalog, &self.params, &self.hooks) {
            Some(result) => result,
            None => Err(PondError::RunnerNotFound.into()),
        }
    }

    /// Dispatch based on the stored [`Command`].
    ///
    /// - `Command::Run` → [`execute`](App::execute)
    /// - `Command::Check` → validate pipeline structure
    /// - `Command::Viz` → build graph and serve/export (std + viz feature only)
    pub fn dispatch<'a, E, F>(&'a self, f: F) -> Result<(), E>
    where
        E: From<PondError> + core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
        F: PipelineFn<'a, C, P, E>,
    {
        match &self.command {
            Command::Run => self.execute(f),
            Command::Check => {
                let pipeline = f.call(&self.catalog, &self.params);
                match pipeline.check() {
                    Ok(()) => {
                        #[cfg(feature = "std")]
                        println!("Pipeline is valid.");
                        Ok(())
                    }
                    Err(e) => {
                        #[cfg(feature = "std")]
                        {
                            Err(PondError::Custom(std::format!(
                                "Pipeline validation failed:\n  - {e}"
                            ))
                            .into())
                        }
                        #[cfg(not(feature = "std"))]
                        {
                            let _ = e;
                            Err(PondError::CheckFailed.into())
                        }
                    }
                }
            }
            #[cfg(feature = "std")]
            Command::Viz { port, output, export } => {
                self.dispatch_viz(f, *port, output.as_deref(), export.as_deref())
            }
        }
    }
}

// --- std-only constructors and methods ---

#[cfg(feature = "std")]
mod std_app {
    use super::*;
    use crate::graph::build_pipeline_graph;
    use clap::Parser;
    use cli::{CliArgs, Command as CliCommand};
    use config::{apply_overrides, deserialize_config, load_yaml};
    use serde::de::DeserializeOwned;

    /// Load a YAML config file, apply overrides, and deserialize.
    fn load_config<T: DeserializeOwned>(
        path: &str,
        overrides: &[String],
    ) -> Result<T, PondError> {
        let mut value = load_yaml(path)?;
        if !overrides.is_empty() {
            apply_overrides(&mut value, overrides);
        }
        Ok(deserialize_config(value)?)
    }

    /// Convert a CLI Command to our core Command, extracting the runner name
    /// and optional node filter.
    fn extract_command(cli_cmd: &CliCommand) -> (Command, Option<String>, Option<crate::pipeline::NodeFilter>) {
        match cli_cmd {
            CliCommand::Run { runner, nodes, from_nodes, to_nodes, .. } => {
                let filter = if !nodes.is_empty() {
                    Some(crate::pipeline::NodeFilter::Nodes(
                        nodes.iter().cloned().collect(),
                    ))
                } else if !from_nodes.is_empty() || !to_nodes.is_empty() {
                    Some(crate::pipeline::NodeFilter::FromTo {
                        from: from_nodes.iter().cloned().collect(),
                        to: to_nodes.iter().cloned().collect(),
                    })
                } else {
                    None
                };
                (Command::Run, runner.clone(), filter)
            }
            CliCommand::Check => (Command::Check, None, None),
            CliCommand::Viz { port, output, export } => (
                Command::Viz {
                    port: *port,
                    output: output.clone(),
                    export: export.clone(),
                },
                None,
                None,
            ),
        }
    }

    impl<C: DeserializeOwned, P: DeserializeOwned>
        App<C, P, (), DefaultRunners>
    {
        /// Create an App from YAML catalog and params files.
        ///
        /// Loads and deserializes both files without any CLI parsing.
        /// Combine with [`with_args`](App::with_args) to add CLI subcommand
        /// dispatch and param overrides.
        ///
        /// # Example
        ///
        /// ```ignore
        /// let app = App::from_yaml("conf/catalog.yml", "conf/params.yml")?
        ///     .with_hooks(my_hooks)
        ///     .with_args(std::env::args_os())?;
        /// app.dispatch(my_pipeline)?;
        /// ```
        pub fn from_yaml(catalog_path: &str, params_path: &str) -> Result<Self, PondError> {
            let catalog: C = load_config(catalog_path, &[])?;
            let params: P = load_config(params_path, &[])?;
            Ok(App {
                catalog,
                params,
                hooks: (),
                runners: DefaultRunners::default(),
                command: Command::Run,
                runner_name: None,
                node_filter: None,
                program_name: String::new(),
            })
        }

        /// Create an App from pre-parsed [`CliArgs`].
        ///
        /// Loads catalog and params from YAML, applies CLI overrides,
        /// and stores the command for [`dispatch`](App::dispatch).
        pub fn from_cli(cli: CliArgs) -> Result<Self, PondError> {
            let catalog_path = cli
                .catalog_path
                .as_deref()
                .unwrap_or("conf/base/catalog.yml");
            let params_path = cli
                .params_path
                .as_deref()
                .unwrap_or("conf/base/parameters.yml");

            let (catalog_overrides, param_overrides) = match &cli.command {
                CliCommand::Run {
                    catalog_overrides,
                    param_overrides,
                    ..
                } => (catalog_overrides.as_slice(), param_overrides.as_slice()),
                _ => (&[][..], &[][..]),
            };

            let catalog: C = load_config(catalog_path, catalog_overrides)?;
            let params: P = load_config(params_path, param_overrides)?;

            let (command, runner_name, node_filter) = extract_command(&cli.command);

            Ok(App {
                catalog,
                params,
                hooks: (),
                runners: DefaultRunners::default(),
                command,
                runner_name,
                node_filter,
                program_name: String::new(),
            })
        }

        /// Create an App by parsing CLI args from an iterator.
        ///
        /// Extracts the program name from the first argument, then
        /// delegates to [`from_cli`](App::from_cli).
        pub fn from_args<I, T>(iter: I) -> Result<Self, PondError>
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let raw_args: Vec<T> = iter.into_iter().collect();
            let program_name: String = raw_args
                .first()
                .map(|a| {
                    let os: std::ffi::OsString = a.clone().into();
                    std::path::Path::new(&os)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| os.to_string_lossy().into_owned())
                })
                .unwrap_or_default();
            let cli = CliArgs::parse_from(raw_args);
            let mut app = Self::from_cli(cli)?;
            app.program_name = program_name;
            Ok(app)
        }
    }

    impl<C, P, H, R> App<C, P, H, R>
    where
        C: Serialize + DeserializeOwned,
        P: Serialize + DeserializeOwned,
    {
        /// Apply CLI overrides to already-provided catalog and params.
        ///
        /// Serializes the current data, patches with CLI overrides,
        /// deserializes back. Stores the command for [`dispatch`](App::dispatch).
        pub fn with_cli(self, cli: CliArgs) -> Result<App<C, P, H, R>, PondError> {
            let (catalog_overrides, param_overrides) = match &cli.command {
                CliCommand::Run {
                    catalog_overrides,
                    param_overrides,
                    ..
                } => (catalog_overrides.as_slice(), param_overrides.as_slice()),
                _ => (&[][..], &[][..]),
            };

            let catalog = if catalog_overrides.is_empty() {
                self.catalog
            } else {
                let mut value = serde_yaml::to_value(&self.catalog)
                    .map_err(PondError::SerdeYaml)?;
                apply_overrides(&mut value, catalog_overrides);
                serde_yaml::from_value(value).map_err(PondError::SerdeYaml)?
            };

            let params = if param_overrides.is_empty() {
                self.params
            } else {
                let mut value = serde_yaml::to_value(&self.params)
                    .map_err(PondError::SerdeYaml)?;
                apply_overrides(&mut value, param_overrides);
                serde_yaml::from_value(value).map_err(PondError::SerdeYaml)?
            };

            let (command, runner_name, node_filter) = extract_command(&cli.command);

            Ok(App {
                catalog,
                params,
                hooks: self.hooks,
                runners: self.runners,
                command,
                runner_name,
                node_filter,
                program_name: self.program_name,
            })
        }
    }

    impl<C: Serialize, P: Serialize + DeserializeOwned, H, R> App<C, P, H, R> {
        /// Parse CLI args and apply command + param overrides to an existing App.
        ///
        /// Unlike [`with_cli`](App::with_cli), this does not require the catalog
        /// to be deserializable — only the params are round-tripped through serde
        /// when overrides are present. Catalog path and overrides are ignored.
        ///
        /// Use this when the catalog is constructed programmatically (e.g.
        /// `RegisterDataset`, `GpioDataset`) but you still want CLI-driven
        /// command selection (`run`, `check`, `viz`) and param overrides.
        pub fn with_args<I, T>(self, iter: I) -> Result<Self, PondError>
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let raw_args: Vec<T> = iter.into_iter().collect();
            let program_name: String = raw_args
                .first()
                .map(|a| {
                    let os: std::ffi::OsString = a.clone().into();
                    std::path::Path::new(&os)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| os.to_string_lossy().into_owned())
                })
                .unwrap_or_default();
            let cli = CliArgs::parse_from(raw_args);

            let param_overrides = match &cli.command {
                CliCommand::Run { param_overrides, .. } => param_overrides.as_slice(),
                _ => &[][..],
            };

            let params = if param_overrides.is_empty() {
                self.params
            } else {
                let mut value = serde_yaml::to_value(&self.params)
                    .map_err(PondError::SerdeYaml)?;
                apply_overrides(&mut value, param_overrides);
                serde_yaml::from_value(value).map_err(PondError::SerdeYaml)?
            };

            let (command, runner_name, node_filter) = extract_command(&cli.command);

            Ok(App {
                catalog: self.catalog,
                params,
                hooks: self.hooks,
                runners: self.runners,
                command,
                runner_name,
                node_filter,
                program_name,
            })
        }
    }

    // --- Viz dispatch (std only) ---

    impl<C: Serialize, P: Serialize, H: Hooks, R: Runners> App<C, P, H, R> {
        pub(super) fn dispatch_viz<'a, E, F>(
            &'a self,
            f: F,
            port: u16,
            output: Option<&str>,
            export: Option<&str>,
        ) -> Result<(), E>
        where
            E: From<PondError> + core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
            F: PipelineFn<'a, C, P, E>,
        {
            let pipeline = f.call(&self.catalog, &self.params);
            let graph = build_pipeline_graph(&pipeline, &self.catalog, &self.params);

            #[cfg(not(feature = "viz"))]
            {
                let _ = (port, output, export, graph);
                Err(PondError::Custom(
                    "viz subcommand requires the 'viz' feature (cargo build --features viz)"
                        .into(),
                )
                .into())
            }

            #[cfg(feature = "viz")]
            {
                use crate::viz::serialization::{collect_dataset_meta, viz_graph_from};
                use crate::viz::assets::FrontendAssets;
                use std::collections::HashMap;

                let mut viz_graph = viz_graph_from(&graph);
                viz_graph.name = self.program_name.clone();
                let dataset_meta = collect_dataset_meta(&graph);

                if let Some(path) = export {
                    // Collect dataset HTML and YAML snapshots
                    let dataset_html: HashMap<usize, String> = dataset_meta.iter()
                        .filter_map(|(&id, meta)| meta.html().map(|h| (id, h)))
                        .collect();
                    let dataset_yaml: HashMap<usize, String> = dataset_meta.iter()
                        .filter_map(|(&id, meta)| meta.yaml().map(|y| (id, y)))
                        .collect();

                    let static_data = serde_json::json!({
                        "graph": viz_graph,
                        "datasetHtml": dataset_html,
                        "datasetYaml": dataset_yaml,
                    });
                    let json = serde_json::to_string(&static_data)
                        .map_err(PondError::from)?;
                    // Escape </script> in JSON to prevent breaking the HTML
                    let json = json.replace("</script", "<\\/script");

                    let template = FrontendAssets::get("index.html")
                        .ok_or_else(|| PondError::Custom("embedded index.html not found".into()))?;
                    let html = String::from_utf8_lossy(&template.data);
                    let script = std::format!(
                        "<script>window.__STATIC_DATA__={};</script>",
                        json
                    );
                    let html_str = html.as_ref();
                    let output_html = match html_str.rfind("</head>") {
                        Some(pos) => std::format!("{}{script}{}", &html_str[..pos], &html_str[pos..]),
                        None => std::format!("{script}{html_str}"),
                    };
                    std::fs::write(path, output_html.as_bytes()).map_err(PondError::from)?;
                    println!("Static HTML exported to {path}");
                    Ok(())
                } else if let Some(path) = output {
                    let json =
                        serde_json::to_string_pretty(&viz_graph).map_err(PondError::from)?;
                    std::fs::write(path, &json).map_err(PondError::from)?;
                    println!("Graph written to {path}");
                    Ok(())
                } else {
                    use crate::viz::server::VizState;
                    use std::sync::Mutex;
                    use tokio::sync::broadcast;

                    let (tx, _rx) = broadcast::channel(64);
                    let state = VizState {
                        graph: viz_graph,
                        dataset_meta,
                        node_statuses: Mutex::new(std::collections::HashMap::new()),
                        dataset_activity: Mutex::new(std::collections::HashMap::new()),
                        tx,
                    };
                    crate::viz::server::start_server(state, port);
                    Ok(())
                }
            }
        }
    }
}
