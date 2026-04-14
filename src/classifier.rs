use std::path::Path;

use crate::{
    model::ItemKind,
    scanner::parser::ParsedFile,
};

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

    /// Rule 5 – Toml / Yaml / Json → Config
    pub fn is_config(parsed: &ParsedFile) -> bool {
        matches!(
            parsed.lang,
            SourceLang::Toml | SourceLang::Yaml | SourceLang::Json
        )
    }

    /// Rule 6 – top_level_kinds contains interface/protocol → Port
    pub fn is_port(parsed: &ParsedFile) -> bool {
        parsed
            .top_level_kinds
            .iter()
            .any(|k| k == "interface_type" || k == "protocol_stmt")
    }

    /// Rule 7 – path contains adapter / integration / provider / backend → Adapter
    pub fn is_adapter(path: &Path) -> bool {
        let s = path.to_string_lossy().to_lowercase();
        s.contains("adapter")
            || s.contains("integration")
            || s.contains("provider")
            || s.contains("backend")
    }

    /// Rule 8 – path contains "main" or "cmd", or top_level_kinds has a
    /// function named "main" → Entrypoint.
    ///
    /// We detect the latter by checking for `function_definition` or
    /// `function_declaration` in `top_level_kinds` combined with "main"
    /// appearing in the raw source.  This is a heuristic, not a full AST
    /// walk, but it is consistent with the no-I/O contract of the classifier.
    pub fn is_entrypoint(parsed: &ParsedFile, path: &Path) -> bool {
        let s = path.to_string_lossy().to_lowercase();
        if s.contains("main") || s.contains("/cmd/") || s.contains("/cmd.") {
            return true;
        }
        let has_main_func = parsed
            .top_level_kinds
            .iter()
            .any(|k| k == "function_definition" || k == "function_declaration");
        has_main_func && parsed.raw_source.contains("def main") || parsed.raw_source.contains("func main(")
    }

    /// Rule 9 – top_level_kinds contains class / type / struct → DomainLogic
    pub fn is_domain_logic(parsed: &ParsedFile) -> bool {
        parsed.top_level_kinds.iter().any(|k| {
            k == "class_definition" || k == "type_declaration" || k == "struct_item"
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
        let (p, path) = make_parsed(
            "src/model.rs",
            SourceLang::Rust,
            vec!["struct_item"],
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
        let (p, path) = make_parsed("tests/fixtures/test_sample.py", SourceLang::Python, vec![], "");
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
