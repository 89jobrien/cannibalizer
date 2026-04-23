use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cnbl",
    about = "Turn any repo into Rust — scan, plan, scaffold, absorb"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Walk a foreign repo and figure out what's in it (outputs JSONL)
    Scan {
        /// Path to the repo you want to cannibalize
        path: PathBuf,

        /// Save the JSONL output to a file instead of printing it
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Also print a summary table showing what was found
        #[arg(long)]
        report: bool,
    },

    /// Decide where each file should go in your Rust ecosystem (outputs JSONL)
    Plan {
        /// Path to your repos.json ecosystem map (default: ~/dev/bazaar/repos.json)
        #[arg(long, value_name = "FILE")]
        repo_map: Option<PathBuf>,

        /// Read scan output from a file instead of stdin
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Show the routing decisions as a table without writing any output
        #[arg(long)]
        dry_run: bool,
    },

    /// Create empty Rust stubs for everything that needs to be ported
    Gen {
        /// Read plan output from a file instead of stdin
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Where to write the generated stubs (default: ./cnbl-output)
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,

        /// Replace existing files if they already exist
        #[arg(long)]
        force: bool,
    },

    /// Copy stubs into your repos and archive the originals — this does the actual work
    Eat {
        /// Read plan output from a file instead of stdin
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Where the generated stubs live (default: ./cnbl-output)
        #[arg(long, value_name = "DIR")]
        scaffold_dir: Option<PathBuf>,

        /// Root directory where your local repos live (default: ~/dev)
        #[arg(long, value_name = "DIR")]
        repo_root: Option<PathBuf>,

        /// Where to archive files that aren't being ported
        #[arg(long, value_name = "DIR")]
        vault_dir: Option<PathBuf>,

        /// Name of the repo being cannibalized
        #[arg(long, value_name = "NAME")]
        source_repo: Option<String>,

        /// Show what would happen without touching anything
        #[arg(long)]
        dry_run: bool,
    },
}
