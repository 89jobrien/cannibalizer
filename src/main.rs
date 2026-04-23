use std::path::PathBuf;

use clap::Parser;

mod classifier;
mod cli;
mod cmd;
mod ecosystem;
mod model;
mod scanner;

fn default_repo_map() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("dev/bazaar/repos.json")
}

fn default_dev_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("dev")
}

fn default_vault_dir() -> PathBuf {
    PathBuf::from("/Volumes/Extreme SSD/vault/cnbl-artifacts")
}

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        cli::Command::Scan {
            path,
            output,
            report,
        } => cmd::scan::run(&path, output.as_deref(), report),
        cli::Command::Plan {
            repo_map,
            input,
            dry_run,
        } => {
            let repo_map = repo_map.unwrap_or_else(default_repo_map);
            cmd::plan::run(input.as_deref(), &repo_map, dry_run)
        }
        cli::Command::Gen {
            input,
            out_dir,
            force,
        } => {
            let out_dir = out_dir.unwrap_or_else(|| PathBuf::from("cnbl-output"));
            cmd::scaffold::run(input.as_deref(), &out_dir, force)
        }
        cli::Command::Eat {
            input,
            scaffold_dir,
            repo_root,
            vault_dir,
            source_repo,
            dry_run,
        } => {
            let scaffold_dir = scaffold_dir.unwrap_or_else(|| PathBuf::from("cnbl-output"));
            let repo_root = repo_root.unwrap_or_else(default_dev_dir);
            let vault_dir = vault_dir.unwrap_or_else(default_vault_dir);
            let source_repo_name = source_repo.unwrap_or_else(|| "unknown".to_string());
            let cfg = cmd::eat::EatConfig {
                input: input.as_deref(),
                scaffold_dir: &scaffold_dir,
                repo_root: &repo_root,
                vault_dir: &vault_dir,
                source_repo_name: &source_repo_name,
                dry_run,
            };
            cmd::eat::run(&cfg)
        }
    }
}
