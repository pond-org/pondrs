//! CLI argument definitions using clap.

use std::prelude::v1::*;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,

    /// Path to catalog YAML config file.
    #[arg(long, global = true)]
    pub catalog_path: Option<String>,

    /// Path to parameters YAML config file.
    #[arg(long, global = true)]
    pub params_path: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Execute the pipeline.
    Run {
        /// Runner to use.
        #[arg(long)]
        runner: Option<RunnerChoice>,

        /// Override parameter values (dot notation for nesting, e.g. model.learning_rate=0.01).
        #[arg(long = "params", value_name = "KEY=VALUE")]
        param_overrides: Vec<String>,

        /// Override catalog values (dot notation for nesting, e.g. output.path=/tmp/out.csv).
        #[arg(long = "catalog", value_name = "KEY=VALUE")]
        catalog_overrides: Vec<String>,
    },

    /// Validate pipeline structure (dependency ordering, output uniqueness).
    Check,

    /// Build pipeline graph and serve visualization (not yet implemented).
    Viz {
        /// Port for the visualization server.
        #[arg(long, default_value = "8080")]
        port: u16,

        /// Write pipeline graph JSON to file instead of serving.
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Clone, ValueEnum)]
pub enum RunnerChoice {
    Sequential,
    Parallel,
    Custom,
}
