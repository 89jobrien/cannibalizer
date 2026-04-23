use std::path::Path;

use crate::{model::ItemKind, scanner::parser::ParsedFile};

pub mod rules {
    use std::path::Path;

    use crate::{model::SourceLang, scanner::parser::ParsedFile};

    /// Rule 1 – path contains "test" or "spec" → TestHarness
    pub fn is_test_harness(path: &Path) -> bool {
        let s = path.to_string_lossy().to_lowercase();
        s.contains("test") || s.contains("spec")
    }

    /// Rule 2 – path contains "fixture" → Discard
    pub fn is_fixture(path: &Path) -> bool {
        path.to_string_lossy().to_lowercase().contains("fixture")
    }

    /// Rule 3 – Shell or Nushell → Script
    pub fn is_script(parsed: &ParsedFile) -> bool {
        matches!(parsed.lang, SourceLang::Shell | SourceLang::Nushell)
    }

    /// Rule 4 – Markdown → Spec
    pub fn is_spec(parsed: &ParsedFile) -> bool {
        matches!(parsed.lang, SourceLang::Markdown)
    }

    /// Rule 5 – Toml / Yaml / Json / Baml → Config
    pub fn is_config(parsed: &ParsedFile) -> bool {
        matches!(
            parsed.lang,
            SourceLang::Toml | SourceLang::Yaml | SourceLang::Json | SourceLang::Baml
        )
    }

    /// Rule 5b – TypeScript / TSX → DomainLogic (classify by lang, no structural analysis).
    ///
    /// TODO(tree-sitter-typescript): once tree-sitter-typescript is added as a dependency,
    /// remove this rule and let rules 6–9 handle TypeScript structurally (ports via
    /// `interface_declaration`, adapters via path, domain logic via `class_declaration` /
    /// `function_declaration` / `arrow_function`).  Until then, all .ts/.tsx files are
    /// conservatively promoted to DomainLogic rather than falling to Discard.
    pub fn is_typescript(parsed: &ParsedFile) -> bool {
        matches!(parsed.lang, SourceLang::TypeScript)
    }

    /// Rule 6 – top_level_kinds contains interface/protocol/trait → Port
    pub fn is_port(parsed: &ParsedFile) -> bool {
        parsed
            .top_level_kinds
            .iter()
            .any(|k| k == "interface_type" || k == "protocol_stmt" || k == "trait_item")
    }

    /// Rule 7 – path contains adapter / integration / provider / backend /
    ///           store / source_ / inbox → Adapter
    pub fn is_adapter(path: &Path) -> bool {
        let s = path.to_string_lossy().to_lowercase();
        s.contains("adapter")
            || s.contains("integration")
            || s.contains("provider")
            || s.contains("backend")
            || s.contains("store_")
            || s.contains("source_")
            || s.contains("/inbox/")
    }

    /// Rule 8 – path stem is "main" or a segment is "cmd", or top_level_kinds has a
    /// function named "main" → Entrypoint.
    ///
    /// We detect the latter by checking for `function_definition` or
    /// `function_declaration` in `top_level_kinds` combined with "main"
    /// appearing in the raw source.  This is a heuristic, not a full AST
    /// walk, but it is consistent with the no-I/O contract of the classifier.
    pub fn is_entrypoint(parsed: &ParsedFile, path: &Path) -> bool {
        // Check file stem exactly rather than substring to avoid "domain" ⊃ "main".
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if stem == "main" {
            return true;
        }
        let s = path.to_string_lossy().to_lowercase();
        if s.contains("/cmd/") || s.contains("/cmd.") {
            return true;
        }
        let has_main_func = parsed
            .top_level_kinds
            .iter()
            .any(|k| k == "function_definition" || k == "function_declaration");
        has_main_func && parsed.raw_source.contains("def main")
            || parsed.raw_source.contains("func main(")
    }

    /// Dispatch-only node kinds across all supported grammars.
    ///
    /// Rust:   use_declaration, mod_item, extern_crate_declaration
    /// Python: import_statement, import_from_statement
    const DISPATCH_KINDS: &[&str] = &[
        "use_declaration",
        "mod_item",
        "extern_crate_declaration",
        "import_statement",
        "import_from_statement",
    ];

    /// Rule 9a – all top-level nodes are pure dispatch (use / mod / extern crate /
    ///           import) → Glue, or file is a known glue filename with no parsed kinds.
    ///
    /// The empty-kinds guard prevents misclassifying truly unknown files as Glue.
    pub fn is_glue(parsed: &ParsedFile, path: &Path) -> bool {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Empty-parse heuristic: known glue filenames
        if parsed.top_level_kinds.is_empty() {
            return matches!(
                name.as_str(),
                "lib.rs" | "mod.rs" | "app.rs" | "__init__.py"
            );
        }

        // All top-level nodes are dispatch-only → Glue
        parsed
            .top_level_kinds
            .iter()
            .all(|k| DISPATCH_KINDS.contains(&k.as_str()))
    }

    /// Rule 9 – top_level_kinds contains class / type / struct / enum / impl /
    ///           function_definition / function_declaration /
    ///           decorated_definition → DomainLogic.
    ///
    /// Python decorated classes (`@dataclass`, `@attrs`) emit `decorated_definition`
    /// at the top level rather than `class_definition`.  Python and Shell modules
    /// are often function-based with no class at the top level.
    ///
    /// Node name provenance:
    /// - `class_definition`    — tree-sitter-python (all versions)
    /// - `decorated_definition`— tree-sitter-python (all versions); wraps @decorator + class/fn
    /// - `type_declaration`    — tree-sitter-go (all versions)
    /// - `function_definition` — tree-sitter-python, tree-sitter-bash
    /// - `function_declaration`— tree-sitter-go
    /// - `function_item`       — tree-sitter-rust (all versions); covers `fn` and `async fn`
    /// - `struct_item`, `enum_item`, `impl_item` — tree-sitter-rust (all versions)
    pub fn is_domain_logic(parsed: &ParsedFile) -> bool {
        parsed.top_level_kinds.iter().any(|k| {
            k == "class_definition"
                || k == "type_declaration"
                || k == "struct_item"
                || k == "enum_item"
                || k == "impl_item"
                || k == "function_item"
                || k == "function_definition"
                || k == "function_declaration"
                || k == "decorated_definition"
        })
    }
}

/// Classify a parsed file using a 10-rule priority chain.
pub fn classify(parsed: &ParsedFile, path: &Path) -> ItemKind {
    // 1
    if rules::is_test_harness(path) {
        return ItemKind::TestHarness;
    }
    // 2
    if rules::is_fixture(path) {
        return ItemKind::Discard;
    }
    // 3
    if rules::is_script(parsed) {
        return ItemKind::Script;
    }
    // 4
    if rules::is_spec(parsed) {
        return ItemKind::Spec;
    }
    // 5
    if rules::is_config(parsed) {
        return ItemKind::Config;
    }
    // 5b
    if rules::is_typescript(parsed) {
        return ItemKind::DomainLogic;
    }
    // 6
    if rules::is_port(parsed) {
        return ItemKind::Port;
    }
    // 7
    if rules::is_adapter(path) {
        return ItemKind::Adapter;
    }
    // 8
    if rules::is_entrypoint(parsed, path) {
        return ItemKind::Entrypoint;
    }
    // 9
    if rules::is_domain_logic(parsed) {
        return ItemKind::DomainLogic;
    }
    // 9a
    if rules::is_glue(parsed, path) {
        return ItemKind::Glue;
    }
    // 10 fallback
    ItemKind::Discard
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::SourceLang;
    use crate::scanner::parser::ParsedFile;

    fn make_parsed(
        path: &str,
        lang: SourceLang,
        top_level_kinds: Vec<&str>,
        raw_source: &str,
    ) -> (ParsedFile, PathBuf) {
        let pb = PathBuf::from(path);
        let parsed = ParsedFile {
            path: pb.clone(),
            lang,
            top_level_kinds: top_level_kinds.into_iter().map(String::from).collect(),
            raw_source: raw_source.to_string(),
        };
        (parsed, pb)
    }

    #[test]
    fn test_path_test_gives_test_harness() {
        let (p, path) = make_parsed("src/test_utils.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::TestHarness));
    }

    #[test]
    fn test_path_spec_gives_test_harness() {
        let (p, path) = make_parsed("src/spec_helpers.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::TestHarness));
    }

    #[test]
    fn fixture_path_gives_discard() {
        // Path has "fixture" but not "test" or "spec"
        let (p, path) = make_parsed("data/fixture_sample.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Discard));
    }

    #[test]
    fn shell_gives_script() {
        let (p, path) = make_parsed("scripts/build.sh", SourceLang::Shell, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Script));
    }

    #[test]
    fn nushell_gives_script() {
        let (p, path) = make_parsed("scripts/deploy.nu", SourceLang::Nushell, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Script));
    }

    #[test]
    fn markdown_gives_spec() {
        let (p, path) = make_parsed("docs/design.md", SourceLang::Markdown, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Spec));
    }

    #[test]
    fn toml_gives_config() {
        let (p, path) = make_parsed("Cargo.toml", SourceLang::Toml, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Config));
    }

    #[test]
    fn yaml_gives_config() {
        let (p, path) = make_parsed(".github/ci.yml", SourceLang::Yaml, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Config));
    }

    #[test]
    fn json_gives_config() {
        let (p, path) = make_parsed("config.json", SourceLang::Json, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Config));
    }

    #[test]
    fn interface_type_gives_port() {
        let (p, path) = make_parsed(
            "src/ports/storage.go",
            SourceLang::Go,
            vec!["interface_type"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::Port));
    }

    #[test]
    fn protocol_stmt_gives_port() {
        let (p, path) = make_parsed(
            "src/ports/cache.py",
            SourceLang::Python,
            vec!["protocol_stmt"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::Port));
    }

    #[test]
    fn adapter_path_gives_adapter() {
        let (p, path) = make_parsed("src/adapter_db.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Adapter));
    }

    #[test]
    fn backend_path_gives_adapter() {
        let (p, path) = make_parsed("src/backend/server.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Adapter));
    }

    #[test]
    fn main_path_gives_entrypoint() {
        let (p, path) = make_parsed("src/main.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Entrypoint));
    }

    #[test]
    fn class_definition_gives_domain_logic() {
        let (p, path) = make_parsed(
            "src/service.py",
            SourceLang::Python,
            vec!["class_definition"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::DomainLogic));
    }

    #[test]
    fn struct_item_gives_domain_logic() {
        let (p, path) = make_parsed("src/model.rs", SourceLang::Rust, vec!["struct_item"], "");
        assert!(matches!(classify(&p, &path), ItemKind::DomainLogic));
    }

    #[test]
    fn decorated_definition_gives_domain_logic() {
        // @dataclass classes emit decorated_definition, not class_definition
        let (p, path) = make_parsed(
            "lib/models/trace.py",
            SourceLang::Python,
            vec!["import_statement", "decorated_definition"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::DomainLogic));
    }

    #[test]
    fn init_py_with_only_imports_gives_glue() {
        let (p, path) = make_parsed(
            "lib/__init__.py",
            SourceLang::Python,
            vec!["import_statement", "import_from_statement"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::Glue));
    }

    #[test]
    fn empty_init_py_gives_glue() {
        let (p, path) = make_parsed("lib/models/__init__.py", SourceLang::Python, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Glue));
    }

    #[test]
    fn typescript_gives_domain_logic() {
        let (p, path) = make_parsed("src/agent.ts", SourceLang::TypeScript, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::DomainLogic));
    }

    #[test]
    fn baml_gives_config() {
        let (p, path) = make_parsed("baml_src/agent.baml", SourceLang::Baml, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Config));
    }

    #[test]
    fn only_use_mod_kinds_gives_glue() {
        // top_level_kinds contains only dispatch nodes → Glue
        let (p, path) = make_parsed(
            "src/lib.rs",
            SourceLang::Rust,
            vec!["use_declaration", "mod_item"],
            "use std::fmt;\nmod inner;",
        );
        assert!(matches!(classify(&p, &path), ItemKind::Glue));
    }

    #[test]
    fn lib_rs_with_empty_kinds_gives_glue() {
        // lib.rs that the parser emitted no top_level_kinds for → Glue by filename
        let (p, path) = make_parsed("src/lib.rs", SourceLang::Rust, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Glue));
    }

    #[test]
    fn mod_rs_with_empty_kinds_gives_glue() {
        // mod.rs outside an adapter path → Glue by filename
        // (avoid paths containing "domain" since "domain" ⊃ "main" — rule 8 would fire first)
        let (p, path) = make_parsed("src/core/mod.rs", SourceLang::Rust, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Glue));
    }

    #[test]
    fn inbox_mod_rs_gives_adapter_not_glue() {
        // /inbox/ in path triggers rule 7 (Adapter) before glue rule — correct
        let (p, path) = make_parsed("src/inbox/mod.rs", SourceLang::Rust, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Adapter));
    }

    #[test]
    fn app_rs_with_empty_kinds_gives_glue() {
        let (p, path) = make_parsed("src/app.rs", SourceLang::Rust, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Glue));
    }

    #[test]
    fn mixed_kinds_does_not_give_glue() {
        // Has a struct alongside use_declaration → DomainLogic, not Glue
        let (p, path) = make_parsed(
            "src/service.rs",
            SourceLang::Rust,
            vec!["use_declaration", "struct_item"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::DomainLogic));
    }

    #[test]
    fn unknown_file_gives_discard() {
        let (p, path) = make_parsed("src/thing.xyz", SourceLang::Unknown, vec![], "");
        assert!(matches!(classify(&p, &path), ItemKind::Discard));
    }

    #[test]
    fn test_path_beats_fixture_path() {
        // "test" appears in path → TestHarness wins over fixture (which is also present)
        // but "fixture" appears too — since rule 1 runs first, TestHarness wins
        let (p, path) = make_parsed(
            "tests/fixtures/test_sample.py",
            SourceLang::Python,
            vec![],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::TestHarness));
    }

    #[test]
    fn adapter_beats_domain_logic() {
        // adapter path takes priority over class_definition structural signal
        let (p, path) = make_parsed(
            "src/adapter_repo.py",
            SourceLang::Python,
            vec!["class_definition"],
            "",
        );
        assert!(matches!(classify(&p, &path), ItemKind::Adapter));
    }
}
