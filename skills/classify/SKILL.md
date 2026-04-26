---
name: classify
description: >
  Debug or inspect classifier output for specific files or paths.
  Shows which rule fired and why. Use when the user asks "why was this
  classified as X", "what kind is this file", "debug classification",
  or wants to understand classifier behavior.
model: haiku
effort: low
allowed-tools:
  - Bash
  - Read
  - Grep
---

# classify

Inspect how the classifier categorizes a specific file or set of files.

## Usage

Scan a single directory or repo and filter the output:

```bash
# Scan and grep for a specific file
cnbl scan <repo> 2>/dev/null | grep '"rel_path":"src/model.py"'

# Scan and show only a specific kind
cnbl scan <repo> 2>/dev/null | grep '"kind":"discard"'

# Scan with full report
cnbl scan <repo> --report 2>&1 >/dev/null
```

## Classifier Rule Chain

The classifier checks rules top-to-bottom. First match wins:

| Priority | Rule            | Kind        | Signal    |
| -------- | --------------- | ----------- | --------- |
| 1        | is_test_harness | TestHarness | path      |
| 2        | is_fixture      | Discard     | path      |
| 3        | is_script       | Script      | lang      |
| 4        | is_spec         | Spec        | lang      |
| 5        | is_config       | Config      | lang      |
| 6        | is_port         | Port        | content   |
| 7        | is_adapter      | Adapter     | path      |
| 8        | is_typescript   | DomainLogic | lang      |
| 9        | is_entrypoint   | Entrypoint  | path+lang |
| 10       | is_domain_logic | DomainLogic | content   |
| 11       | is_glue         | Glue        | content   |
| --       | fallthrough     | Discard     | --        |

## Debugging Tips

- If a file is unexpectedly Discard, check if it has a recognized
  extension (walker.rs `lang_from_ext`)
- If a TS file is DomainLogic when it should be Adapter, check if
  the path contains an adapter keyword (rule 7 runs before rule 8)
- If a Rust file with content is Discard, check if tree-sitter
  parsed it (zero top_level_kinds = parse failure or unsupported syntax)
- Path rules use substring matching -- "test" in any path component
  triggers TestHarness (rule 1)

## Source

Classifier implementation: `src/classifier.rs`
Conformance tests: `tests/conformance_classifier.rs`
Spec: `specs/classifier-hardening.md`
