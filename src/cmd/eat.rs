//! `cnbl eat` — copy stubs into your repos and archive the originals.

use std::{
    io::{self, BufRead},
    path::Path,
};

use anyhow::Context as _;

use crate::ecosystem::{Destination, RouteDecision};

// ============================================================================
// COMMAND
// ============================================================================

pub struct EatConfig<'a> {
    pub input: Option<&'a Path>,
    pub scaffold_dir: &'a Path,
    pub repo_root: &'a Path,
    pub vault_dir: &'a Path,
    pub source_repo_name: &'a str,
    pub dry_run: bool,
}

pub fn run(cfg: &EatConfig<'_>) -> anyhow::Result<()> {
    let decisions = read_plan(cfg.input)?;

    let mut errors: Vec<String> = Vec::new();
    let mut actions_taken: Vec<String> = Vec::new();

    for decision in &decisions {
        match &decision.destination {
            Destination::Archive => {
                let src = &decision.item.rel_path;
                // Strip leading `/` so absolute paths become relative when
                // appended to the vault root.
                let rel = src.strip_prefix("/").unwrap_or(src);
                let dest = cfg.vault_dir.join(cfg.source_repo_name).join(rel);
                if cfg.dry_run {
                    println!("archive  {}  →  {}", src.display(), dest.display());
                } else {
                    eprintln!("  archive {} → {}", src.display(), dest.display());
                    match copy_preserving(src, &dest) {
                        Ok(()) => actions_taken.push(format!("archived {}", src.display())),
                        Err(e) => errors.push(format!("archive {}: {e}", src.display())),
                    }
                }
            }
            Destination::ExistingRepo { name, .. }
            | Destination::NewCrate {
                suggested_name: name,
            } => {
                let scaffold_src = cfg.scaffold_dir.join(name);
                let repo_dest = cfg.repo_root.join(name);
                if !repo_dest.exists() {
                    eprintln!(
                        "warn: skipping '{}' — not found at {}",
                        name,
                        repo_dest.display()
                    );
                    continue;
                }
                if cfg.dry_run {
                    println!(
                        "copy scaffold  {}  →  {}",
                        scaffold_src.display(),
                        repo_dest.display()
                    );
                } else {
                    eprintln!(
                        "  copy {} → {}",
                        scaffold_src.display(),
                        repo_dest.display()
                    );
                    match copy_dir_all(&scaffold_src, &repo_dest) {
                        Ok(()) => actions_taken
                            .push(format!("copied scaffold to {}", repo_dest.display())),
                        Err(e) => {
                            errors.push(format!("copy scaffold to {}: {e}", repo_dest.display()))
                        }
                    }
                }
            }
            Destination::Discard => {
                if cfg.dry_run {
                    println!("discard  {} (no action)", decision.item.rel_path.display());
                }
            }
        }
    }

    if cfg.dry_run {
        return Ok(());
    }

    eprintln!("\nExec summary: {} action(s) taken", actions_taken.len());
    for a in &actions_taken {
        eprintln!("  ok  {a}");
    }

    if !errors.is_empty() {
        eprintln!("\n{} error(s):", errors.len());
        for e in &errors {
            eprintln!("  err {e}");
        }
        anyhow::bail!("cnbl exec completed with {} error(s)", errors.len());
    }

    Ok(())
}

// ============================================================================
// HELPERS
// ============================================================================

fn read_plan(input: Option<&Path>) -> anyhow::Result<Vec<RouteDecision>> {
    let lines: Box<dyn Iterator<Item = io::Result<String>>> = match input {
        Some(path) => {
            let f = std::fs::File::open(path)
                .with_context(|| format!("cannot open plan file: {}", path.display()))?;
            Box::new(io::BufReader::new(f).lines())
        }
        None => Box::new(io::BufReader::new(io::stdin()).lines()),
    };

    let mut decisions = Vec::new();
    for (i, line) in lines.enumerate() {
        let line = line.with_context(|| format!("read error on line {i}"))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let d: RouteDecision = serde_json::from_str(line)
            .with_context(|| format!("invalid plan JSON on line {i}: {line}"))?;
        decisions.push(d);
    }
    Ok(decisions)
}

fn copy_preserving(src: &Path, dest: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create dir: {}", parent.display()))?;
    }
    std::fs::copy(src, dest)
        .with_context(|| format!("cannot copy {} → {}", src.display(), dest.display()))?;
    Ok(())
}

fn copy_dir_all(src: &Path, dest: &Path) -> anyhow::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dest)
        .with_context(|| format!("cannot create dir: {}", dest.display()))?;
    for entry in
        std::fs::read_dir(src).with_context(|| format!("cannot read dir: {}", src.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)
                .with_context(|| format!("cannot copy file: {}", entry.path().display()))?;
        }
    }
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        ecosystem::{Destination, RouteDecision},
        model::{HarvestItem, ItemKind, SourceLang},
    };

    fn make_decision(kind: ItemKind, path: &str, dest: Destination) -> RouteDecision {
        RouteDecision {
            item: HarvestItem {
                rel_path: PathBuf::from(path),
                lang: SourceLang::Rust,
                kind,
                size_bytes: 10,
                notes: None,
            },
            destination: dest,
            rationale: "test".to_string(),
        }
    }

    #[test]
    fn dry_run_does_not_copy_files() {
        let tmp = tempfile::tempdir().unwrap();

        let decisions = vec![make_decision(
            ItemKind::DomainLogic,
            "src/lib.rs",
            Destination::ExistingRepo {
                name: "doob".to_string(),
                url: "https://github.com/89jobrien/doob".to_string(),
            },
        )];

        let plan_file = tmp.path().join("plan.jsonl");
        {
            use std::io::Write as _;
            let mut f = std::fs::File::create(&plan_file).unwrap();
            for d in &decisions {
                writeln!(f, "{}", serde_json::to_string(d).unwrap()).unwrap();
            }
        }

        let scaffold_dir = tmp.path().join("scaffold");
        let repo_root = tmp.path().join("repos");
        let vault_dir = tmp.path().join("vault");

        let cfg = EatConfig {
            input: Some(&plan_file),
            scaffold_dir: &scaffold_dir,
            repo_root: &repo_root,
            vault_dir: &vault_dir,
            source_repo_name: "my-repo",
            dry_run: true,
        };

        // Should not error even though dirs don't exist — dry-run.
        run(&cfg).unwrap();

        // Nothing should have been created.
        assert!(!scaffold_dir.exists());
        assert!(!vault_dir.exists());
    }

    #[test]
    fn archive_copies_file_to_vault() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a fake source file.
        let source_dir = tmp.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        let src_file = source_dir.join("config.toml");
        std::fs::write(&src_file, "[package]").unwrap();

        let decisions = vec![make_decision(
            ItemKind::Config,
            src_file.to_str().unwrap(),
            Destination::Archive,
        )];

        let plan_file = tmp.path().join("plan.jsonl");
        {
            use std::io::Write as _;
            let mut f = std::fs::File::create(&plan_file).unwrap();
            for d in &decisions {
                writeln!(f, "{}", serde_json::to_string(d).unwrap()).unwrap();
            }
        }

        let vault_dir = tmp.path().join("vault");
        let scaffold_dir = tmp.path().join("scaffold");
        let repo_root = tmp.path().join("repos");

        let cfg = EatConfig {
            input: Some(&plan_file),
            scaffold_dir: &scaffold_dir,
            repo_root: &repo_root,
            vault_dir: &vault_dir,
            source_repo_name: "my-repo",
            dry_run: false,
        };

        run(&cfg).unwrap();

        let rel = src_file.strip_prefix("/").unwrap_or(&src_file);
        let expected = vault_dir.join("my-repo").join(rel);
        assert!(
            expected.exists(),
            "vault copy should exist at {}",
            expected.display()
        );
    }
}
