---
name: scan
description: >
  Run cnbl scan on a target repo to classify every file by kind
  (domain logic, port, adapter, script, spec, config, glue, discard).
  Outputs JSONL. Use when the user wants to "scan a repo", "see what's
  in a project", or "classify files".
model: haiku
effort: low
allowed-tools:
  - Bash
  - Read
---

# scan

Run the scanner on a target repo and display the classification report.

## Usage

```bash
cnbl scan <repo-path> --output /tmp/cnbl-scan.jsonl --report
```

## Output

JSONL stream where each line is a `HarvestItem`:

```json
{
  "rel_path": "src/model.py",
  "lang": "python",
  "kind": "domain_logic",
  "size_bytes": 1234,
  "notes": null
}
```

## Kinds

| Kind         | Meaning                                       |
| ------------ | --------------------------------------------- |
| domain_logic | Types, structs, classes worth porting         |
| port         | Interfaces, traits, protocols                 |
| adapter      | Infrastructure implementations                |
| entrypoint   | main files, cmd/ directories                  |
| test_harness | Test files (discarded during planning)        |
| script       | Shell/Nu scripts                              |
| spec         | Markdown docs                                 |
| config       | TOML/YAML/JSON/BAML config files              |
| glue         | Re-export files (lib.rs, **init**.py, mod.rs) |
| discard      | Unrecognized or fixture files                 |

## Report

With `--report`, a summary table prints to stderr showing counts per
kind and the languages found in each category. Use this to get a
quick overview before running `plan`.

## Notes

- Files > 512KB are skipped (logged to stderr)
- Tree-sitter grammars: Python, Go, Shell, Rust
- TypeScript classified by lang only (no grammar)
- Ignored dirs: .git, node_modules, target, **pycache**, .venv, dist
