use anyhow::bail;
use clap::Parser;

mod classifier;
mod cli;
mod cmd;
mod model;
mod scanner;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        cli::Command::Scan { path, output, report } => {
            cmd::scan::run(&path, output.as_deref(), report)
        }
        cli::Command::Plan { .. } => bail!("plan: not yet implemented"),
        cli::Command::Gen { .. } => bail!("gen: not yet implemented"),
        cli::Command::Exec => bail!("exec: not yet implemented"),
    }
}
