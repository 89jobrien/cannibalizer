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

        /// Print a human-readable harvest report to stderr after the JSONL stream
        #[arg(long)]
        report: bool,
    },

    /// Route classified harvest items to ecosystem destinations
    Plan {
        /// Path to repos.json repo-map file
        #[arg(long, value_name = "FILE")]
        repo_map: Option<PathBuf>,

        /// Read harvest JSONL from this file instead of stdin
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Print a human-readable plan table instead of JSONL
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate hexagonal scaffold stubs from a migration plan
    Gen {
        /// Read plan JSONL from this file instead of stdin
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Directory to write generated output into (default: ./cnbl-output)
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,

        /// Overwrite existing files without aborting
        #[arg(long)]
        force: bool,
    },

    /// Execute a migration plan: copy scaffolds and archive files
    Exec {
        /// Read plan JSONL from this file instead of stdin
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Directory containing cnbl gen output (default: ./cnbl-output)
        #[arg(long, value_name = "DIR")]
        scaffold_dir: Option<PathBuf>,

        /// Root directory where local repos live (default: ~/dev)
        #[arg(long, value_name = "DIR")]
        repo_root: Option<PathBuf>,

        /// Vault directory for archived files
        #[arg(long, value_name = "DIR")]
        vault_dir: Option<PathBuf>,

        /// Name of the source repo being cannibalized
        #[arg(long, value_name = "NAME")]
        source_repo: Option<String>,

        /// Print what would be done without doing anything
        #[arg(long)]
        dry_run: bool,
    },
}
