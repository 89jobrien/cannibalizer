//! Ecosystem routing — maps `HarvestItem`s to destination repos or new crates.

use std::path::Path;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::model::{HarvestItem, ItemKind};

// ============================================================================
// DOMAIN TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemRepo {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Destination {
    ExistingRepo { name: String, url: String },
    NewCrate { suggested_name: String },
    Archive,
    Discard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub item: HarvestItem,
    pub destination: Destination,
    pub rationale: String,
}

// ============================================================================
// LOADING
// ============================================================================

/// Load repos from a JSON file. Returns an empty vec (with a warning) if the
/// file is missing rather than propagating an error.
pub fn load_repos(path: &Path) -> anyhow::Result<Vec<EcosystemRepo>> {
    if !path.exists() {
        eprintln!(
            "warn: repo-map not found at {}, using empty list",
            path.display()
        );
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read repo-map: {}", path.display()))?;
    let repos: Vec<EcosystemRepo> = serde_json::from_str(&raw)
        .with_context(|| format!("cannot parse repo-map: {}", path.display()))?;
    Ok(repos)
}

// ============================================================================
// ROUTING
// ============================================================================

/// Score a repo against a set of keywords from the item path.
fn score_repo(repo: &EcosystemRepo, keywords: &[&str]) -> usize {
    let haystack = format!(
        "{} {}",
        repo.name,
        repo.description.as_deref().unwrap_or("")
    )
    .to_lowercase();
    keywords.iter().filter(|kw| haystack.contains(**kw)).count()
}

/// Find the best-matching repo for a set of keywords. Returns `None` if no
/// repo scores above zero.
pub fn domain_match<'a>(
    keywords: &[&str],
    repos: &'a [EcosystemRepo],
) -> Option<&'a EcosystemRepo> {
    repos
        .iter()
        .map(|r| (r, score_repo(r, keywords)))
        .filter(|(_, score)| *score > 0)
        .max_by_key(|(_, score)| *score)
        .map(|(r, _)| r)
}

/// Apply the routing rules from the issue spec and return a `RouteDecision`.
pub fn route(item: HarvestItem, repos: &[EcosystemRepo]) -> RouteDecision {
    let path_str = item.rel_path.to_string_lossy().to_lowercase();

    // Fixed-destination rules first.
    match item.kind {
        ItemKind::Discard => {
            return RouteDecision {
                item,
                destination: Destination::Discard,
                rationale: "item kind is Discard".to_string(),
            };
        }
        ItemKind::TestHarness => {
            return RouteDecision {
                item,
                destination: Destination::Discard,
                rationale: "tests stay with their source repo".to_string(),
            };
        }
        ItemKind::Config => {
            return RouteDecision {
                item,
                destination: Destination::Archive,
                rationale: "config files are archived".to_string(),
            };
        }
        ItemKind::Script => {
            if path_str.contains("harvest")
                || path_str.contains("sync")
                || path_str.contains("ingest")
            {
                return repo_decision(
                    item,
                    "harvestrs",
                    repos,
                    "script path matches harvest/sync/ingest",
                );
            }
            if path_str.contains("hook")
                || path_str.contains("course")
                || path_str.contains("block")
            {
                return repo_decision(
                    item,
                    "coursers",
                    repos,
                    "script path matches hook/course/block",
                );
            }
            if path_str.contains("fmt") || path_str.contains("format") || path_str.contains("lint")
            {
                return repo_decision(item, "fmtx", repos, "script path matches fmt/format/lint");
            }
            return RouteDecision {
                item,
                destination: Destination::Archive,
                rationale: "script with no matching ecosystem target".to_string(),
            };
        }
        ItemKind::Spec if path_str.contains("todo") || path_str.contains("task") => {
            return repo_decision(item, "doob", repos, "spec path matches todo/task");
        }
        _ => {}
    }

    // For DomainLogic/Port/Adapter/Spec/Entrypoint — keyword match against repos.
    let path_keywords: Vec<&str> = item
        .rel_path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if let Some(repo) = domain_match(&path_keywords, repos) {
        return RouteDecision {
            destination: Destination::ExistingRepo {
                name: repo.name.clone(),
                url: repo.url.clone(),
            },
            rationale: format!("keyword match on repo '{}'", repo.name),
            item,
        };
    }

    // Final fallback — suggest a new crate from the last path segment.
    let suggested = item
        .rel_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_lowercase()
        .replace('-', "_");

    RouteDecision {
        destination: Destination::NewCrate {
            suggested_name: suggested.clone(),
        },
        rationale: format!("no ecosystem match; suggested new crate '{suggested}'"),
        item,
    }
}

/// Helper: look up a named repo and build an ExistingRepo destination, or fall
/// back to NewCrate if the repo isn't in the list.
fn repo_decision(
    item: HarvestItem,
    repo_name: &str,
    repos: &[EcosystemRepo],
    rationale: &str,
) -> RouteDecision {
    if let Some(repo) = repos.iter().find(|r| r.name == repo_name) {
        RouteDecision {
            destination: Destination::ExistingRepo {
                name: repo.name.clone(),
                url: repo.url.clone(),
            },
            rationale: rationale.to_string(),
            item,
        }
    } else {
        RouteDecision {
            destination: Destination::NewCrate {
                suggested_name: repo_name.to_string(),
            },
            rationale: format!("{rationale} (repo not in map)"),
            item,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{ItemKind, SourceLang};

    fn fixture_repos() -> Vec<EcosystemRepo> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repos.json");
        load_repos(&path).unwrap()
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
    fn discard_kind_routes_to_discard() {
        let repos = fixture_repos();
        let d = route(item(ItemKind::Discard, "junk.txt"), &repos);
        assert!(matches!(d.destination, Destination::Discard));
    }

    #[test]
    fn test_harness_routes_to_discard() {
        let repos = fixture_repos();
        let d = route(item(ItemKind::TestHarness, "tests/test_foo.py"), &repos);
        assert!(matches!(d.destination, Destination::Discard));
    }

    #[test]
    fn config_routes_to_archive() {
        let repos = fixture_repos();
        let d = route(item(ItemKind::Config, "Cargo.toml"), &repos);
        assert!(matches!(d.destination, Destination::Archive));
    }

    #[test]
    fn harvest_script_routes_to_harvestrs() {
        let repos = fixture_repos();
        let d = route(item(ItemKind::Script, "scripts/harvest_notes.sh"), &repos);
        assert!(
            matches!(d.destination, Destination::ExistingRepo { ref name, .. } if name == "harvestrs")
        );
    }

    #[test]
    fn hook_script_routes_to_coursers() {
        let repos = fixture_repos();
        let d = route(item(ItemKind::Script, "scripts/hook_block.sh"), &repos);
        assert!(
            matches!(d.destination, Destination::ExistingRepo { ref name, .. } if name == "coursers")
        );
    }

    #[test]
    fn fmt_script_routes_to_fmtx() {
        let repos = fixture_repos();
        let d = route(item(ItemKind::Script, "tools/fmt_check.sh"), &repos);
        assert!(
            matches!(d.destination, Destination::ExistingRepo { ref name, .. } if name == "fmtx")
        );
    }

    #[test]
    fn todo_spec_routes_to_doob() {
        let repos = fixture_repos();
        let i = HarvestItem {
            rel_path: PathBuf::from("specs/todo_format.md"),
            lang: SourceLang::Markdown,
            kind: ItemKind::Spec,
            size_bytes: 100,
            notes: None,
        };
        let d = route(i, &repos);
        assert!(
            matches!(d.destination, Destination::ExistingRepo { ref name, .. } if name == "doob")
        );
    }

    #[test]
    fn unknown_item_suggests_new_crate() {
        let repos = fixture_repos();
        let i = HarvestItem {
            rel_path: PathBuf::from("src/auth_service.py"),
            lang: SourceLang::Python,
            kind: ItemKind::DomainLogic,
            size_bytes: 200,
            notes: None,
        };
        let d = route(i, &repos);
        assert!(matches!(d.destination, Destination::NewCrate { .. }));
    }

    #[test]
    fn load_repos_missing_file_returns_empty() {
        let repos = load_repos(Path::new("/nonexistent/repos.json")).unwrap();
        assert!(repos.is_empty());
    }

    #[test]
    fn load_repos_fixture_file_parses() {
        let repos = fixture_repos();
        assert!(!repos.is_empty());
        assert!(repos.iter().any(|r| r.name == "harvestrs"));
    }
}
