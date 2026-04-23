use std::path::{Path, PathBuf};

use crate::model::SourceLang;

const MAX_FILE_BYTES: u64 = 512 * 1024;

pub struct ParsedFile {
    #[allow(dead_code)]
    pub path: PathBuf,
    pub lang: SourceLang,
    pub top_level_kinds: Vec<String>,
    pub raw_source: String,
}

/// Parse a source file with tree-sitter and extract top-level node kinds.
///
/// Files larger than 512 KB are skipped — an empty `ParsedFile` is returned
/// with `lang` set to `SourceLang::Unknown`.
///
/// Languages without a tree-sitter grammar (`Unknown`, `Markdown`, `Toml`,
/// `Yaml`, `Json`, `Nushell`) bypass parsing and return an empty
/// `top_level_kinds` vec.
///
/// Parse errors are returned as `Err` and should be logged by the caller.
pub fn parse_file(path: &Path, lang: SourceLang) -> anyhow::Result<ParsedFile> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_FILE_BYTES {
        log::warn!(
            "skipping {}: file size {} exceeds 512 KB limit",
            path.display(),
            metadata.len()
        );
        return Ok(ParsedFile {
            path: path.to_path_buf(),
            lang: SourceLang::Unknown,
            top_level_kinds: vec![],
            raw_source: String::new(),
        });
    }

    let raw_source = std::fs::read_to_string(path)?;

    let ts_lang: Option<tree_sitter::Language> = match lang {
        SourceLang::Python => Some(tree_sitter_python::LANGUAGE.into()),
        SourceLang::Go => Some(tree_sitter_go::LANGUAGE.into()),
        SourceLang::Shell => Some(tree_sitter_bash::LANGUAGE.into()),
        SourceLang::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
        _ => None,
    };

    let top_level_kinds = if let Some(ts_lang) = ts_lang {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&ts_lang)
            .map_err(|e| anyhow::anyhow!("failed to set language: {e}"))?;

        let tree = parser.parse(raw_source.as_bytes(), None).ok_or_else(|| {
            anyhow::anyhow!("tree-sitter returned no tree for {}", path.display())
        })?;

        let root = tree.root_node();
        let mut cursor = root.walk();
        root.children(&mut cursor)
            .map(|child| child.kind().to_string())
            .collect()
    } else {
        vec![]
    };

    Ok(ParsedFile {
        path: path.to_path_buf(),
        lang,
        top_level_kinds,
        raw_source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn parse_python_extracts_top_level_kinds() {
        let path = fixture("sample.py");
        let result = parse_file(&path, SourceLang::Python).unwrap();
        assert!(matches!(result.lang, SourceLang::Python));
        // sample.py has import_statement, class_definition, function_definition
        assert!(
            result
                .top_level_kinds
                .iter()
                .any(|k| k == "import_statement"),
            "expected import_statement in {:?}",
            result.top_level_kinds
        );
        assert!(
            result
                .top_level_kinds
                .iter()
                .any(|k| k == "class_definition"),
            "expected class_definition in {:?}",
            result.top_level_kinds
        );
        assert!(
            result
                .top_level_kinds
                .iter()
                .any(|k| k == "function_definition"),
            "expected function_definition in {:?}",
            result.top_level_kinds
        );
    }

    #[test]
    fn parse_go_extracts_top_level_kinds() {
        let path = fixture("sample.go");
        let result = parse_file(&path, SourceLang::Go).unwrap();
        assert!(matches!(result.lang, SourceLang::Go));
        // sample.go has type_declaration, function_declaration, method_declaration
        assert!(
            result
                .top_level_kinds
                .iter()
                .any(|k| k == "function_declaration" || k == "method_declaration"),
            "expected function_declaration or method_declaration in {:?}",
            result.top_level_kinds
        );
    }

    #[test]
    fn parse_shell_extracts_top_level_kinds() {
        let path = fixture("sample.sh");
        let result = parse_file(&path, SourceLang::Shell).unwrap();
        assert!(matches!(result.lang, SourceLang::Shell));
        assert!(
            result
                .top_level_kinds
                .iter()
                .any(|k| k == "function_definition"),
            "expected function_definition in {:?}",
            result.top_level_kinds
        );
    }

    #[test]
    fn parse_non_ts_lang_returns_empty_kinds() {
        let _path = fixture("sample.py"); // reuse any file, lang overrides
        // Use Toml lang — no grammar, should return empty kinds
        // We need a toml fixture but let's write a temp one
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "[package]\nname = \"foo\"").unwrap();
        let result = parse_file(tmp.path(), SourceLang::Toml).unwrap();
        assert!(matches!(result.lang, SourceLang::Toml));
        assert!(result.top_level_kinds.is_empty());
    }

    #[test]
    fn parse_large_file_returns_unknown() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        // Write > 512 KB
        let chunk = b"# padding\n".repeat(60_000);
        tmp.write_all(&chunk).unwrap();
        let result = parse_file(tmp.path(), SourceLang::Python).unwrap();
        assert!(matches!(result.lang, SourceLang::Unknown));
        assert!(result.top_level_kinds.is_empty());
    }

    #[test]
    fn parse_missing_file_returns_err() {
        let result = parse_file(Path::new("/nonexistent/path/file.py"), SourceLang::Python);
        assert!(result.is_err());
    }

    #[test]
    fn parse_rust_struct_emits_struct_item() {
        use std::io::Write as _;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "pub struct Foo {{ pub x: u32 }}").unwrap();
        let result = parse_file(tmp.path(), SourceLang::Rust).unwrap();
        assert!(
            result.top_level_kinds.iter().any(|k| k == "struct_item"),
            "expected struct_item, got {:?}",
            result.top_level_kinds
        );
    }

    #[test]
    fn parse_rust_trait_emits_trait_item() {
        use std::io::Write as _;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "pub trait MyPort {{ fn do_it(&self); }}").unwrap();
        let result = parse_file(tmp.path(), SourceLang::Rust).unwrap();
        assert!(
            result.top_level_kinds.iter().any(|k| k == "trait_item"),
            "expected trait_item, got {:?}",
            result.top_level_kinds
        );
    }

    #[test]
    fn parse_rust_actual_model_file() {
        let path = std::path::PathBuf::from("/Users/joe/dev/kan/kan-core/src/model.rs");
        if !path.exists() {
            return;
        }
        let result = parse_file(&path, SourceLang::Rust).unwrap();
        eprintln!("kan model.rs kinds: {:?}", result.top_level_kinds);
        assert!(
            result.top_level_kinds.iter().any(|k| k == "struct_item"),
            "expected struct_item, got {:?}",
            result.top_level_kinds
        );
    }
}
