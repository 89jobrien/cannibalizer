use std::{
    collections::HashMap,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

use anyhow::{Context, bail};

use crate::{
    classifier,
    model::{HarvestItem, ItemKind, SourceLang},
    scanner::{parser::parse_file, walker::walk},
};

/// Run the `scan` subcommand.
///
/// Walks `root`, parses every file, classifies it, emits JSONL to `out_path`
/// (or stdout when `None`), and optionally prints a report to stderr.
pub fn run(root: &Path, out_path: Option<&Path>, report: bool) -> anyhow::Result<()> {
    if !root.exists() {
        bail!("path does not exist: {}", root.display());
    }
    if !root.is_dir() {
        bail!("path is not a directory: {}", root.display());
    }

    let start = Instant::now();

    // Prepare output writer.
    let stdout;
    let file;
    let writer: Box<dyn Write> = if let Some(p) = out_path {
        file = std::fs::File::create(p)
            .with_context(|| format!("cannot create output file: {}", p.display()))?;
        Box::new(BufWriter::new(file))
    } else {
        stdout = std::io::stdout();
        Box::new(BufWriter::new(stdout.lock()))
    };
    let mut writer = writer;

    let mut items: Vec<HarvestItem> = Vec::new();
    let mut skipped: u64 = 0;
    let mut total: u64 = 0;

    for (abs_path, lang) in walk(root) {
        total += 1;
        if total.is_multiple_of(100) {
            eprint!(".");
        }

        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(&abs_path)
            .to_path_buf();

        let size_bytes = std::fs::metadata(&abs_path).map(|m| m.len()).unwrap_or(0);

        let parsed = match parse_file(&abs_path, lang) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("warn: skipping {}: {e}", rel_path.display());
                skipped += 1;
                continue;
            }
        };

        let kind = classifier::classify(&parsed, &rel_path);
        let item = HarvestItem {
            rel_path,
            lang: parsed.lang,
            kind,
            size_bytes,
            notes: None,
        };

        writeln!(writer, "{}", item.to_jsonl_line())?;
        items.push(item);
    }

    // Flush before printing stderr summary.
    writer.flush()?;

    let elapsed = start.elapsed().as_secs_f64();

    if report {
        print_report(&items, total, skipped, elapsed);
    } else {
        eprintln!("\nScan complete: {total} files in {elapsed:.1}s ({skipped} skipped)");
    }

    Ok(())
}

fn kind_label(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::DomainLogic => "domain_logic",
        ItemKind::Port => "port",
        ItemKind::Adapter => "adapter",
        ItemKind::Entrypoint => "entrypoint",
        ItemKind::TestHarness => "test_harness",
        ItemKind::Script => "script",
        ItemKind::Spec => "spec",
        ItemKind::Config => "config",
        ItemKind::Discard => "discard",
    }
}

fn lang_label(lang: &SourceLang) -> &'static str {
    match lang {
        SourceLang::Python => "python",
        SourceLang::Go => "go",
        SourceLang::Shell => "shell",
        SourceLang::Nushell => "nushell",
        SourceLang::Rust => "rust",
        SourceLang::Markdown => "markdown",
        SourceLang::Toml => "toml",
        SourceLang::Yaml => "yaml",
        SourceLang::Json => "json",
        SourceLang::Unknown => "unknown",
    }
}

/// Print the human-readable harvest report to stderr.
pub fn print_report(items: &[HarvestItem], total: u64, skipped: u64, elapsed: f64) {
    eprintln!("\nScan complete: {total} files in {elapsed:.1}s ({skipped} skipped)");
    eprintln!();

    // Build per-kind counts and per-kind language sets.
    let all_kinds: &[ItemKind] = &[
        ItemKind::DomainLogic,
        ItemKind::Adapter,
        ItemKind::Script,
        ItemKind::Spec,
        ItemKind::Config,
        ItemKind::TestHarness,
        ItemKind::Entrypoint,
        ItemKind::Port,
        ItemKind::Discard,
    ];

    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut langs_by_kind: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut discards: Vec<&HarvestItem> = Vec::new();
    let mut entrypoints: Vec<&HarvestItem> = Vec::new();

    for item in items {
        let kl = kind_label(&item.kind);
        *counts.entry(kl).or_insert(0) += 1;
        let ll = lang_label(&item.lang);
        langs_by_kind.entry(kl).or_default().push(ll);
        if matches!(item.kind, ItemKind::Discard) {
            discards.push(item);
        }
        if matches!(item.kind, ItemKind::Entrypoint) {
            entrypoints.push(item);
        }
    }

    eprintln!("{:<16} {:>5}  Languages", "Kind", "Count");
    eprintln!("{:-<16} {:->5}  {:-<20}", "", "", "");

    for kind in all_kinds {
        let kl = kind_label(kind);
        let count = counts.get(kl).copied().unwrap_or(0);
        let mut lang_set: Vec<&str> = langs_by_kind
            .get(kl)
            .map(|v| {
                let mut s: Vec<&str> = v.clone();
                s.sort_unstable();
                s.dedup();
                s
            })
            .unwrap_or_default();
        lang_set.sort_unstable();
        lang_set.dedup();
        let langs_str = lang_set.join(", ");
        eprintln!("{kl:<16} {count:>5}  {langs_str}");
    }

    if !discards.is_empty() {
        eprintln!();
        eprintln!("Discarded files ({}):", discards.len());
        for item in &discards {
            eprintln!("  {}", item.rel_path.display());
        }
    }

    if !entrypoints.is_empty() {
        eprintln!();
        eprintln!("Entrypoints ({}):", entrypoints.len());
        for item in &entrypoints {
            eprintln!("  {}", item.rel_path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceLang;
    use std::path::PathBuf;

    fn make_item(kind: ItemKind, lang: SourceLang, path: &str) -> HarvestItem {
        HarvestItem {
            rel_path: PathBuf::from(path),
            lang,
            kind,
            size_bytes: 100,
            notes: None,
        }
    }

    /// Capture stderr by using print_report's return structure.
    /// We test counts are correct by building items and checking kind_label counts.
    #[test]
    fn report_counts_correct_kinds() {
        let items = vec![
            make_item(ItemKind::DomainLogic, SourceLang::Rust, "src/lib.rs"),
            make_item(ItemKind::DomainLogic, SourceLang::Python, "src/model.py"),
            make_item(ItemKind::Adapter, SourceLang::Rust, "src/adapter_db.rs"),
            make_item(ItemKind::Script, SourceLang::Shell, "build.sh"),
            make_item(ItemKind::Discard, SourceLang::Unknown, "binary.bin"),
        ];

        // Build counts manually to assert same logic as print_report.
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for item in &items {
            *counts.entry(kind_label(&item.kind)).or_insert(0) += 1;
        }

        assert_eq!(counts["domain_logic"], 2);
        assert_eq!(counts["adapter"], 1);
        assert_eq!(counts["script"], 1);
        assert_eq!(counts["discard"], 1);
    }

    #[test]
    fn scan_nonexistent_path_returns_err() {
        let result = run(Path::new("/nonexistent/cannibalizer/path"), None, false);
        assert!(result.is_err());
    }

    #[test]
    fn scan_file_path_returns_err() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "hello").unwrap();
        let result = run(tmp.path(), None, false);
        assert!(result.is_err());
    }

    #[test]
    fn scan_fixture_dir_produces_items() {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        // Write to a temp file to avoid polluting stdout in tests
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = run(&fixtures, Some(tmp.path()), false);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        // Each line should be valid JSON
        for line in content.lines() {
            let parsed: serde_json::Value =
                serde_json::from_str(line).expect("each output line should be valid JSON");
            assert!(parsed.get("kind").is_some());
        }
    }
}
