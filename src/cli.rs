use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cnbl", about = "Absorb foreign repos into the Rust ecosystem")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan a foreign repository and classify its files
    Scan {
        /// Path to the repository root to scan
        path: PathBuf,

        /// Write output to this file (default: stdout)
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Build a migration plan from a repo-map file
    Plan {
        /// Path to repos.json repo-map file
        #[arg(long, value_name = "FILE", default_value = "~/dev/bazaar/repos.json")]
        repo_map: PathBuf,
    },

    /// Generate Rust scaffolding from a migration plan
    Gen {
        /// Directory to write generated output into
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
    },

    /// Execute a migration plan [not yet implemented]
    Exec,
}
