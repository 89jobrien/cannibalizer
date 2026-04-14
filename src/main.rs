use anyhow::bail;
use clap::Parser;

#[allow(dead_code)]
mod cli;
#[allow(dead_code)]
mod model;
#[allow(dead_code)]
mod scanner;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        cli::Command::Scan { .. } => bail!("scan: not yet implemented"),
        cli::Command::Plan { .. } => bail!("plan: not yet implemented"),
        cli::Command::Gen { .. } => bail!("gen: not yet implemented"),
        cli::Command::Exec => bail!("exec: not yet implemented"),
    }
}
