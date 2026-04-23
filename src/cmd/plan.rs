//! `cnbl plan` — route classified harvest items to ecosystem destinations.

use std::{
    io::{self, BufRead, BufWriter, Write},
    path::Path,
};

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::{
    ecosystem::{self, Destination, RouteDecision},
    model::HarvestItem,
};

// ============================================================================
// PLAN TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub items: Vec<RouteDecision>,
    pub summary: PlanSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub total: usize,
    pub existing_repo: usize,
    pub new_crate: usize,
    pub archive: usize,
    pub discard: usize,
}

// ============================================================================
// COMMAND
// ============================================================================

pub fn run(input: Option<&Path>, repo_map: &Path, dry_run: bool) -> anyhow::Result<()> {
    let repos = ecosystem::load_repos(repo_map)?;
    let items = read_harvest_items(input)?;
    let decisions: Vec<RouteDecision> = items
        .into_iter()
        .map(|item| ecosystem::route(item, &repos))
        .collect();

    let summary = build_summary(&decisions);
    let plan = MigrationPlan {
        items: decisions,
        summary,
    };

    if dry_run {
        print_dry_run(&plan);
    } else {
        emit_jsonl(&plan)?;
        print_summary_stderr(&plan);
    }

    Ok(())
}

fn read_harvest_items(input: Option<&Path>) -> anyhow::Result<Vec<HarvestItem>> {
    let lines: Box<dyn Iterator<Item = io::Result<String>>> = match input {
        Some(path) => {
            let f = std::fs::File::open(path)
                .with_context(|| format!("cannot open input file: {}", path.display()))?;
            Box::new(io::BufReader::new(f).lines())
        }
        None => Box::new(io::BufReader::new(io::stdin()).lines()),
    };

    let mut items = Vec::new();
    for (i, line) in lines.enumerate() {
        let line = line.with_context(|| format!("error reading line {i}"))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let item: HarvestItem = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON on line {i}: {line}"))?;
        items.push(item);
    }
    Ok(items)
}

fn build_summary(decisions: &[RouteDecision]) -> PlanSummary {
    let mut existing_repo = 0;
    let mut new_crate = 0;
    let mut archive = 0;
    let mut discard = 0;
    for d in decisions {
        match &d.destination {
            Destination::ExistingRepo { .. } => existing_repo += 1,
            Destination::NewCrate { .. } => new_crate += 1,
            Destination::Archive => archive += 1,
            Destination::Discard => discard += 1,
        }
    }
    PlanSummary {
        total: decisions.len(),
        existing_repo,
        new_crate,
        archive,
        discard,
    }
}

fn emit_jsonl(plan: &MigrationPlan) -> anyhow::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for decision in &plan.items {
        let line = serde_json::to_string(decision)?;
        writeln!(out, "{line}")?;
    }
    out.flush()?;
    Ok(())
}

fn print_summary_stderr(plan: &MigrationPlan) {
    let s = &plan.summary;
    eprintln!("\nPlan summary");
    eprintln!();

    // Collect new-crate names for display.
    let new_crate_names: Vec<String> = plan
        .items
        .iter()
        .filter_map(|d| match &d.destination {
            Destination::NewCrate { suggested_name } => Some(suggested_name.clone()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Per-repo counts.
    let mut repo_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for d in &plan.items {
        if let Destination::ExistingRepo { name, .. } = &d.destination {
            *repo_counts.entry(name.clone()).or_insert(0) += 1;
        }
    }

    let mut repo_names: Vec<&String> = repo_counts.keys().collect();
    repo_names.sort();
    for name in repo_names {
        let count = repo_counts[name];
        eprintln!(
            "→ {name:<16} {count} item{}",
            if count == 1 { "" } else { "s" }
        );
    }

    if s.archive > 0 {
        eprintln!(
            "→ {:<16} {} item{}",
            "archive",
            s.archive,
            if s.archive == 1 { "" } else { "s" }
        );
    }
    if s.discard > 0 {
        eprintln!(
            "→ {:<16} {} item{}",
            "discard",
            s.discard,
            if s.discard == 1 { "" } else { "s" }
        );
    }
    if s.new_crate > 0 {
        let names_str = new_crate_names.join(", ");
        eprintln!(
            "→ {:<16} {} item{}  (suggested: {})",
            "new crate",
            s.new_crate,
            if s.new_crate == 1 { "" } else { "s" },
            names_str
        );
    }
}

fn print_dry_run(plan: &MigrationPlan) {
    println!("Plan summary (dry-run)");
    println!();
    for d in &plan.items {
        let dest = match &d.destination {
            Destination::ExistingRepo { name, .. } => format!("→ {name}"),
            Destination::NewCrate { suggested_name } => format!("→ new:{suggested_name}"),
            Destination::Archive => "→ archive".to_string(),
            Destination::Discard => "→ discard".to_string(),
        };
        println!("{:<50} {dest}", d.item.rel_path.display());
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        ecosystem::EcosystemRepo,
        model::{ItemKind, SourceLang},
    };

    fn make_repos() -> Vec<EcosystemRepo> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repos.json");
        ecosystem::load_repos(&path).unwrap()
    }

    fn item(kind: ItemKind, path: &str) -> HarvestItem {
        HarvestItem {
            rel_path: PathBuf::from(path),
            lang: SourceLang::Shell,
            kind,
            size_bytes: 10,
            notes: None,
        }
    }

    #[test]
    fn summary_counts_correct() {
        let repos = make_repos();
        let items = vec![
            item(ItemKind::Discard, "junk.bin"),
            item(ItemKind::Config, "Cargo.toml"),
            item(ItemKind::Script, "scripts/harvest.sh"),
        ];
        let decisions: Vec<RouteDecision> = items
            .into_iter()
            .map(|i| ecosystem::route(i, &repos))
            .collect();
        let s = build_summary(&decisions);
        assert_eq!(s.total, 3);
        assert_eq!(s.discard, 1);
        assert_eq!(s.archive, 1);
        assert_eq!(s.existing_repo, 1);
    }

    #[test]
    fn read_harvest_items_from_file() {
        use std::io::Write as _;

        let item = HarvestItem {
            rel_path: PathBuf::from("src/lib.rs"),
            lang: SourceLang::Rust,
            kind: ItemKind::DomainLogic,
            size_bytes: 42,
            notes: None,
        };
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "{}", serde_json::to_string(&item).unwrap()).unwrap();

        let result = read_harvest_items(Some(tmp.path())).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].kind, ItemKind::DomainLogic));
    }
}
