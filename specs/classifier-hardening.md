# Classifier Hardening -- Regression Tests & Rule Precedence

## Summary

Add regression tests and minor structural improvements to
`src/classifier.rs` following the df6437a fixes (empty-Rust Glue gate,
TS catch-all reorder). The existing test suite covers the happy paths
but lacks coverage for the specific edge cases those fixes addressed.

## Motivation

Council analysis (2026-04-26, meta-score 0.85) identified four test
gaps in the classifier after recent correctness fixes. The fixes are
correct but unprotected -- a future rule insertion could silently
reintroduce the misclassifications they solved.

## Test Gaps

### T1: Rust empty-file vs parse-failure distinction

The `is_glue` rule now gates on `raw_source.trim().is_empty()` for
Rust files with zero top-level kinds. No test exercises the negative
case (non-empty Rust, zero kinds).

**Add tests:**

```rust
#[test]
fn nonempty_rust_zero_kinds_gives_discard() {
    // Non-empty Rust file that produced zero top-level kinds
    // (parse failure or unsupported syntax) should NOT become Glue.
    let (p, path) = make_parsed(
        "src/weird.rs",
        SourceLang::Rust,
        vec![],
        "some content the parser didn't understand",
    );
    assert!(matches!(classify(&p, &path), ItemKind::Discard));
}

#[test]
fn whitespace_only_rust_gives_glue() {
    // Whitespace-only Rust file counts as empty -- Glue is acceptable.
    let (p, path) = make_parsed(
        "src/placeholder.rs",
        SourceLang::Rust,
        vec![],
        "   \n\n  \t  ",
    );
    assert!(matches!(classify(&p, &path), ItemKind::Glue));
}
```

**Decision needed:** Should whitespace-only Rust files be Glue or
Discard? Current behavior: Glue (trim makes them empty). If we want
Discard, change the gate to `raw_source.is_empty()` instead of
`raw_source.trim().is_empty()`. Recommend keeping current behavior --
whitespace-only files are effectively empty stubs.

### T2: TypeScript under adapter/infra paths

The TS catch-all was moved below Port/Adapter detection, but no test
verifies a TS file in an adapter path classifies as Adapter (not
DomainLogic).

**Add tests:**

```rust
#[test]
fn typescript_in_adapter_path_gives_adapter() {
    let (p, path) = make_parsed(
        "src/adapters/api_client.ts",
        SourceLang::TypeScript,
        vec![],
        "",
    );
    assert!(matches!(classify(&p, &path), ItemKind::Adapter));
}

#[test]
fn typescript_in_infra_path_gives_adapter() {
    let (p, path) = make_parsed(
        "src/infra/database.ts",
        SourceLang::TypeScript,
        vec![],
        "",
    );
    assert!(matches!(classify(&p, &path), ItemKind::Adapter));
}

#[test]
fn typescript_with_port_content_gives_port() {
    // TS file with interface_type should be Port, not DomainLogic
    let (p, path) = make_parsed(
        "src/ports/storage.ts",
        SourceLang::TypeScript,
        vec!["interface_type"],
        "",
    );
    assert!(matches!(classify(&p, &path), ItemKind::Port));
}

#[test]
fn typescript_generic_path_gives_domain_logic() {
    // TS file NOT in adapter/port path still falls to DomainLogic
    let (p, path) = make_parsed(
        "src/service.ts",
        SourceLang::TypeScript,
        vec![],
        "",
    );
    assert!(matches!(classify(&p, &path), ItemKind::DomainLogic));
}
```

### T3: Fuzz corpus for classifier inputs

The existing fuzz target covers `RepoInfo` deserialization but not
classifier behavior. Add a fuzz target that constructs arbitrary
`ParsedFile` + path combinations and asserts `classify` never panics.

**Add fuzz target** (`fuzz/fuzz_targets/classifier.rs`):

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct ClassifierInput {
    path: String,
    lang: u8,
    kinds: Vec<String>,
    raw_source: String,
}

fuzz_target!(|input: ClassifierInput| {
    let lang = match input.lang % 10 {
        0 => cnbl::model::SourceLang::Rust,
        1 => cnbl::model::SourceLang::Python,
        2 => cnbl::model::SourceLang::Go,
        3 => cnbl::model::SourceLang::TypeScript,
        4 => cnbl::model::SourceLang::Shell,
        5 => cnbl::model::SourceLang::Nushell,
        6 => cnbl::model::SourceLang::Markdown,
        7 => cnbl::model::SourceLang::Toml,
        8 => cnbl::model::SourceLang::Yaml,
        _ => cnbl::model::SourceLang::Unknown,
    };
    let parsed = cnbl::scanner::parser::ParsedFile {
        path: std::path::PathBuf::from(&input.path),
        lang,
        top_level_kinds: input.kinds,
        raw_source: input.raw_source,
    };
    let path = std::path::Path::new(&input.path);
    let _ = cnbl::classifier::classify(&parsed, path);
});
```

## Structural Improvement

### S1: Rule precedence documentation

The `classify` function encodes precedence as linear if-return order.
The numbered comments (1, 2, 3...) are already present but the TS
catch-all is labeled "5b" which breaks the numbering. Renumber the
rules sequentially and add a precedence table as a doc comment on
`classify`:

```rust
/// Classify a parsed file using a priority chain:
///
/// | Priority | Rule            | Kind         | Signal    |
/// |----------|-----------------|--------------|-----------|
/// | 1        | is_test_harness | TestHarness  | path      |
/// | 2        | is_fixture      | Discard      | path      |
/// | 3        | is_script       | Script       | lang      |
/// | 4        | is_spec         | Spec         | lang      |
/// | 5        | is_config       | Config       | lang      |
/// | 6        | is_port         | Port         | content   |
/// | 7        | is_adapter      | Adapter      | path      |
/// | 8        | is_typescript   | DomainLogic  | lang      |
/// | 9        | is_entrypoint   | Entrypoint   | path+lang |
/// | 10       | is_domain_logic | DomainLogic  | content   |
/// | 11       | is_glue         | Glue         | content   |
/// | --       | fallthrough     | Discard      | --        |
```

This makes the ordering auditable without reading the function body.

## Conformance Tests

Classifier conformance tests live in `tests/conformance_classifier.rs`
and are parameterized over `classify`. They assert invariants that must
hold for ANY valid input, not just specific examples. The existing unit
tests cover "input X produces output Y"; conformance tests cover
"for all inputs satisfying property P, the output satisfies property Q".

### Design

```rust
use std::path::Path;
use cnbl::classifier::classify;
use cnbl::model::{ItemKind, SourceLang};
use cnbl::scanner::parser::ParsedFile;

fn make(
    path: &str,
    lang: SourceLang,
    kinds: Vec<&str>,
    raw: &str,
) -> ParsedFile {
    ParsedFile {
        path: path.into(),
        lang,
        top_level_kinds: kinds.into_iter().map(String::from).collect(),
        raw_source: raw.to_string(),
    }
}
```

### C1: Determinism

Calling `classify` twice on the same input returns the same result.

```rust
#[test]
fn c1_deterministic() {
    let cases = [
        make("src/lib.rs", SourceLang::Rust, vec!["mod_item"], "mod x;"),
        make("src/main.py", SourceLang::Python, vec![], ""),
        make("src/agent.ts", SourceLang::TypeScript, vec![], ""),
        make("src/weird.rs", SourceLang::Rust, vec![], "unparseable"),
        make("docs/api.md", SourceLang::Markdown, vec![], "# API"),
    ];
    for p in &cases {
        let path = Path::new(p.path.to_str().unwrap());
        let a = classify(p, path);
        let b = classify(p, path);
        assert_eq!(
            std::mem::discriminant(&a),
            std::mem::discriminant(&b),
            "classify must be deterministic for {:?}",
            p.path,
        );
    }
}
```

### C2: Exhaustive output coverage

Every `ItemKind` variant is reachable. This prevents dead variants
and catches rule ordering bugs that shadow an entire category.

```rust
#[test]
fn c2_all_item_kinds_reachable() {
    let cases: Vec<(ParsedFile, ItemKind)> = vec![
        (make("tests/foo.rs", Rust, vec![], ""), ItemKind::TestHarness),
        (make("data/fixture.py", Python, vec![], ""), ItemKind::Discard),
        (make("run.sh", Shell, vec![], ""), ItemKind::Script),
        (make("README.md", Markdown, vec![], ""), ItemKind::Spec),
        (make("Cargo.toml", Toml, vec![], ""), ItemKind::Config),
        (make("src/port.go", Go, vec!["interface_type"], ""), ItemKind::Port),
        (make("src/adapter_db.py", Python, vec![], ""), ItemKind::Adapter),
        (make("src/main.rs", Rust, vec![], ""), ItemKind::Entrypoint),
        (make("src/model.rs", Rust, vec!["struct_item"], ""), ItemKind::DomainLogic),
        (make("src/lib.rs", Rust, vec!["use_declaration"], "use x;"), ItemKind::Glue),
        (make("src/x.xyz", Unknown, vec![], ""), ItemKind::Discard),
    ];
    let mut seen: std::collections::HashSet<String> = Default::default();
    for (p, expected) in &cases {
        let path = Path::new(p.path.to_str().unwrap());
        let got = classify(p, path);
        assert_eq!(
            std::mem::discriminant(&got),
            std::mem::discriminant(expected),
            "{:?}: expected {:?}, got {:?}",
            p.path, expected, got,
        );
        seen.insert(format!("{:?}", got));
    }
    // Every non-Discard variant must appear (Discard appears via
    // both fixture and fallthrough, so count unique variants)
    assert!(
        seen.len() >= 9,
        "expected all 9 distinct ItemKind variants reachable, got {}",
        seen.len(),
    );
}
```

### C3: Path-based rules are case-insensitive

All path-matching rules (`is_test_harness`, `is_fixture`,
`is_adapter`, `is_entrypoint`) lowercase the path before matching.
Conformance: mixed-case paths produce the same result as lowercase.

```rust
#[test]
fn c3_path_rules_case_insensitive() {
    let pairs = [
        ("src/Test_utils.py", "src/test_utils.py"),
        ("data/FIXTURE_data.py", "data/fixture_data.py"),
        ("src/Adapter_repo.go", "src/adapter_repo.go"),
        ("src/Backend/server.py", "src/backend/server.py"),
        ("src/Main.py", "src/main.py"),
    ];
    for (upper, lower) in &pairs {
        let pu = make(upper, SourceLang::Python, vec![], "");
        let pl = make(lower, SourceLang::Python, vec![], "");
        let ru = classify(&pu, Path::new(upper));
        let rl = classify(&pl, Path::new(lower));
        assert_eq!(
            std::mem::discriminant(&ru),
            std::mem::discriminant(&rl),
            "case mismatch: {:?} -> {:?}, {:?} -> {:?}",
            upper, ru, lower, rl,
        );
    }
}
```

### C4: Lang-only rules are path-independent

Rules that match purely on `SourceLang` (Script, Spec, Config, TS
catch-all) must produce the same result regardless of path, as long
as the path doesn't trigger a higher-priority path rule.

```rust
#[test]
fn c4_lang_rules_path_independent() {
    let neutral_paths = ["src/foo.sh", "lib/bar.sh", "deep/nested/baz.sh"];
    for path_str in &neutral_paths {
        let p = make(path_str, SourceLang::Shell, vec![], "");
        assert!(
            matches!(classify(&p, Path::new(path_str)), ItemKind::Script),
            "Shell file at {} should be Script",
            path_str,
        );
    }
    let neutral_paths = ["docs/a.md", "lib/b.md", "x.md"];
    for path_str in &neutral_paths {
        let p = make(path_str, SourceLang::Markdown, vec![], "");
        assert!(
            matches!(classify(&p, Path::new(path_str)), ItemKind::Spec),
            "Markdown at {} should be Spec",
            path_str,
        );
    }
}
```

### C5: Higher-priority rules always win

For every adjacent pair of rules (N, N+1), construct an input that
matches both and verify rule N wins.

```rust
#[test]
fn c5_precedence_pairs() {
    // Each tuple: (path, lang, kinds, raw, expected_winner)
    let cases: Vec<(&str, SourceLang, Vec<&str>, &str, ItemKind)> = vec![
        // R1 > R2: path has both "test" and "fixture"
        ("tests/fixtures/helper.py", Python, vec![], "",
         ItemKind::TestHarness),
        // R1 > R3: test path + Shell lang
        ("tests/run.sh", Shell, vec![], "",
         ItemKind::TestHarness),
        // R2 > R3: fixture path + Shell lang
        ("data/fixture_run.sh", Shell, vec![], "",
         ItemKind::Discard),
        // R6 > R7: Port content + adapter path
        ("src/adapter_port.go", Go, vec!["interface_type"], "",
         ItemKind::Port),
        // R7 > R8: adapter path + TS lang
        ("src/adapters/client.ts", TypeScript, vec![], "",
         ItemKind::Adapter),
        // R8 > R9: TS lang + entrypoint path
        // (TS catch-all fires before entrypoint check)
        ("src/main.ts", TypeScript, vec![], "",
         ItemKind::DomainLogic),
        // R7 > R10: adapter path + domain-logic content
        ("src/adapter_repo.py", Python, vec!["class_definition"], "",
         ItemKind::Adapter),
        // R10 > R11: domain-logic content + glue-only kinds
        // (struct_item is domain-logic, so it wins over all-dispatch check)
        ("src/lib.rs", Rust, vec!["struct_item", "use_declaration"],
         "use x;\nstruct Foo;",
         ItemKind::DomainLogic),
    ];
    for (path, lang, kinds, raw, expected) in &cases {
        let p = make(path, *lang, kinds.clone(), raw);
        let got = classify(&p, Path::new(path));
        assert_eq!(
            std::mem::discriminant(&got),
            std::mem::discriminant(expected),
            "precedence: {:?} expected {:?}, got {:?}",
            path, expected, got,
        );
    }
}
```

### C6: Empty input safety

Classifier never panics on degenerate inputs: empty path, empty
kinds, empty raw_source, all combinations.

```rust
#[test]
fn c6_empty_inputs_no_panic() {
    let langs = [
        SourceLang::Rust, SourceLang::Python, SourceLang::Go,
        SourceLang::TypeScript, SourceLang::Shell, SourceLang::Nushell,
        SourceLang::Markdown, SourceLang::Toml, SourceLang::Yaml,
        SourceLang::Json, SourceLang::Baml, SourceLang::Unknown,
    ];
    for lang in &langs {
        // Totally empty
        let p = make("", *lang, vec![], "");
        let _ = classify(&p, Path::new(""));

        // Empty path, non-empty content
        let p = make("", *lang, vec!["struct_item"], "struct Foo;");
        let _ = classify(&p, Path::new(""));

        // Valid path, empty everything else
        let p = make("src/foo.rs", *lang, vec![], "");
        let _ = classify(&p, Path::new("src/foo.rs"));
    }
}
```

### C7: Unknown kinds don't promote

An input with only unrecognized `top_level_kinds` strings (not in
the domain-logic set, not in the dispatch set) must not classify as
DomainLogic or Glue -- it should fall through to Discard (assuming
no path/lang rule matches).

```rust
#[test]
fn c7_unknown_kinds_give_discard() {
    let p = make(
        "src/foo.rs",
        SourceLang::Rust,
        vec!["completely_made_up_kind", "another_fake_kind"],
        "some content",
    );
    assert!(matches!(
        classify(&p, Path::new("src/foo.rs")),
        ItemKind::Discard,
    ));
}
```

### C8: Rust empty-file gate

Non-empty Rust files with zero top-level kinds must NOT become Glue.
This is the specific regression guard for the df6437a fix.

```rust
#[test]
fn c8_nonempty_rust_zero_kinds_not_glue() {
    let contents = [
        "fn main() {}",                          // valid but unparsed
        "some content the parser missed",         // garbage
        "   \n\n  \t  ",                          // whitespace-only (Glue OK)
    ];
    for raw in &contents {
        let p = make("src/mystery.rs", SourceLang::Rust, vec![], raw);
        let result = classify(&p, Path::new("src/mystery.rs"));
        if raw.trim().is_empty() {
            // Whitespace-only: Glue is acceptable
            assert!(
                matches!(result, ItemKind::Glue),
                "whitespace-only Rust should be Glue, got {:?}",
                result,
            );
        } else {
            // Non-empty: must NOT be Glue
            assert!(
                !matches!(result, ItemKind::Glue),
                "non-empty Rust with zero kinds should not be Glue, got {:?}",
                result,
            );
        }
    }
}
```

### C9: TS adapter/port precedence

TypeScript files under adapter paths classify as Adapter, not
DomainLogic. TS files with port content classify as Port. Regression
guard for the TS catch-all reorder.

```rust
#[test]
fn c9_typescript_path_precedence() {
    // Adapter paths
    let adapter_paths = [
        "src/adapters/client.ts",
        "src/backend/api.ts",
        "src/integration/hook.ts",
        "src/provider/auth.ts",
    ];
    for path in &adapter_paths {
        let p = make(path, SourceLang::TypeScript, vec![], "");
        assert!(
            matches!(classify(&p, Path::new(path)), ItemKind::Adapter),
            "TS at {} should be Adapter",
            path,
        );
    }

    // Port content wins over TS catch-all
    let p = make(
        "src/ports/repo.ts",
        SourceLang::TypeScript,
        vec!["interface_type"],
        "",
    );
    assert!(matches!(
        classify(&p, Path::new("src/ports/repo.ts")),
        ItemKind::Port,
    ));

    // Generic TS path still falls to DomainLogic
    let p = make("src/service.ts", SourceLang::TypeScript, vec![], "");
    assert!(matches!(
        classify(&p, Path::new("src/service.ts")),
        ItemKind::DomainLogic,
    ));
}
```

### Summary Table

| ID  | Invariant                           | Type       |
| --- | ----------------------------------- | ---------- |
| C1  | Determinism                         | universal  |
| C2  | All ItemKind variants reachable     | coverage   |
| C3  | Path rules are case-insensitive     | universal  |
| C4  | Lang rules are path-independent     | universal  |
| C5  | Higher-priority rules always win    | precedence |
| C6  | Empty/degenerate inputs don't panic | safety     |
| C7  | Unknown kinds fall to Discard       | safety     |
| C8  | Non-empty Rust zero-kinds not Glue  | regression |
| C9  | TS adapter/port path precedence     | regression |

## Non-Goals

- No new classification rules or categories.
- No refactoring classify into a rule table or phased pipeline (the
  council suggested this but it's premature -- the linear chain is
  clear enough with <15 rules).
- No parse-confidence signal or "unparsed Rust" bucket (interesting
  idea but no current consumer needs it).
- No logging/metrics for parse failures (can revisit if
  misclassification reports increase).

## Dependencies

- `arbitrary` + `derive` (dev-dependency, for classifier fuzz target)
- `libfuzzer-sys` (fuzz target only)

## Estimated Scope

- 1 file modified: `src/classifier.rs` (tests + doc comment)
- 1 file created: `fuzz/fuzz_targets/classifier.rs`
- No API changes, no new modules.
