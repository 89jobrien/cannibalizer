use std::path::{Path, PathBuf};

use crate::model::SourceLang;

fn lang_from_ext(ext: &str) -> SourceLang {
    match ext {
        "py" => SourceLang::Python,
        "go" => SourceLang::Go,
        "sh" | "bash" => SourceLang::Shell,
        "nu" => SourceLang::Nushell,
        "rs" => SourceLang::Rust,
        "md" => SourceLang::Markdown,
        "toml" => SourceLang::Toml,
        "yaml" | "yml" => SourceLang::Yaml,
        "json" => SourceLang::Json,
        _ => SourceLang::Unknown,
    }
}

static IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "dist",
];

static IGNORED_FILES: &[&str] = &[
    ".DS_Store",
    "Cargo.lock",
    "uv.lock",
    "package-lock.json",
    "go.sum",
];

/// Walk `root` recursively, yielding `(PathBuf, SourceLang)` for every file
/// that passes the ignore rules. Symlinks are not followed.
pub fn walk(root: &Path) -> impl Iterator<Item = (PathBuf, SourceLang)> {
    walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.file_type().is_dir() {
                return !IGNORED_DIRS.contains(&name.as_ref());
            }
            true
        })
        .filter_map(|res| {
            let entry = res.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy();
            if IGNORED_FILES.contains(&name.as_ref()) {
                return None;
            }
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let lang = lang_from_ext(&ext);
            Some((entry.into_path(), lang))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(dir: &Path, rel: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, "").unwrap();
    }

    #[test]
    fn detects_languages_from_extension() {
        assert!(matches!(lang_from_ext("py"), SourceLang::Python));
        assert!(matches!(lang_from_ext("go"), SourceLang::Go));
        assert!(matches!(lang_from_ext("sh"), SourceLang::Shell));
        assert!(matches!(lang_from_ext("bash"), SourceLang::Shell));
        assert!(matches!(lang_from_ext("nu"), SourceLang::Nushell));
        assert!(matches!(lang_from_ext("rs"), SourceLang::Rust));
        assert!(matches!(lang_from_ext("md"), SourceLang::Markdown));
        assert!(matches!(lang_from_ext("toml"), SourceLang::Toml));
        assert!(matches!(lang_from_ext("yaml"), SourceLang::Yaml));
        assert!(matches!(lang_from_ext("yml"), SourceLang::Yaml));
        assert!(matches!(lang_from_ext("json"), SourceLang::Json));
        assert!(matches!(lang_from_ext("xyz"), SourceLang::Unknown));
    }

    #[test]
    fn skips_ignored_directories() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        touch(root, "main.rs");
        touch(root, ".git/config");
        touch(root, "node_modules/foo/index.js");
        touch(root, "target/debug/cnbl");
        touch(root, "__pycache__/mod.pyc");
        touch(root, ".venv/lib/site.py");
        touch(root, "dist/bundle.js");

        let results: Vec<_> = walk(root).collect();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].1, SourceLang::Rust));
    }

    #[test]
    fn skips_ignored_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        touch(root, "main.rs");
        touch(root, "Cargo.lock");
        touch(root, "uv.lock");
        touch(root, "package-lock.json");
        touch(root, "go.sum");
        touch(root, ".DS_Store");

        let results: Vec<_> = walk(root).collect();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].1, SourceLang::Rust));
    }

    #[test]
    fn emits_correct_langs_for_mixed_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        touch(root, "script.py");
        touch(root, "main.go");
        touch(root, "build.sh");
        touch(root, "config.toml");
        touch(root, "README.md");
        touch(root, "data.json");

        let results: Vec<_> = walk(root).collect();
        let has = |l: &str| {
            results.iter().any(|(_, lang)| format!("{lang:?}") == l)
        };
        assert!(has("Python"));
        assert!(has("Go"));
        assert!(has("Shell"));
        assert!(has("Toml"));
        assert!(has("Markdown"));
        assert!(has("Json"));
    }

    #[test]
    fn does_not_follow_symlinks() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        touch(root, "real.rs");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = TempDir::new().unwrap();
            touch(target.path(), "hidden.rs");
            symlink(target.path(), root.join("linked")).unwrap();
        }

        let results: Vec<_> = walk(root).collect();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].1, SourceLang::Rust));
    }
}
