# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## Build & Test

```bash
cargo build                    # compile
cargo test                     # all unit + integration tests
cargo test --test <name>       # single integration test file
cargo test classifier          # filter by test name substring
cargo clippy                   # lint
cargo fmt --check              # format check
```

Rust edition 2024. Single-crate binary (`cnbl`), no workspace.

## CLI Pipeline

cnbl has a 4-stage pipeline where each stage reads JSONL from the
previous stage (stdin or `--input FILE`):

```
scan  -->  plan  -->  gen  -->  eat
```

| Command | Input          | Output          | Side effects             |
| ------- | -------------- | --------------- | ------------------------ |
| `scan`  | directory path | JSONL to stdout | none                     |
| `plan`  | scan JSONL     | JSONL to stdout | none                     |
| `gen`   | plan JSONL     | stub files      | writes to `cnbl-output/` |
| `eat`   | plan JSONL     | nothing         | copies stubs, archives   |

Example: `cnbl scan ~/some-repo --output scan.jsonl`
then `cnbl plan --input scan.jsonl --repo-map repos.json --dry-run`

## Architecture

```
src/
  main.rs          -- CLI entrypoint, default paths
  cli.rs           -- clap arg definitions (Command enum)
  model.rs         -- SourceLang, ItemKind, HarvestItem (JSONL record)
  classifier.rs    -- 11-rule priority chain: ParsedFile + path -> ItemKind
  ecosystem.rs     -- EcosystemRepo, Destination, route() scoring
  scanner/
    walker.rs      -- recursive walk with ignore rules, lang detection
    parser.rs      -- tree-sitter parsing -> ParsedFile (top_level_kinds)
  cmd/
    scan.rs        -- walk + parse + classify -> JSONL
    plan.rs        -- load repos.json, route each item -> JSONL
    scaffold.rs    -- emit hexagonal Rust stubs (domain/, ports/, adapters/)
    eat.rs         -- copy stubs into repos, archive originals to vault
```

### Data flow

`walker::walk()` yields `(PathBuf, SourceLang)` per file.
`parser::parse_file()` runs tree-sitter and returns `ParsedFile`
(path, lang, top_level_kinds, raw_source).
`classifier::classify()` applies an ordered priority chain to produce
`ItemKind`. `ecosystem::route()` maps `HarvestItem` to a
`RouteDecision` (ExistingRepo / NewCrate / Archive / Discard).

### Classifier rule chain

The classifier is a linear priority chain in `classify()`. Rules are
checked top-to-bottom; first match wins. Three signal types:

- **path**: substring match on file path (test, fixture, adapter, etc.)
- **lang**: match on `SourceLang` enum (Shell->Script, Markdown->Spec)
- **content**: match on `top_level_kinds` from tree-sitter parse

Rule ordering matters. The TS catch-all (rule 8) must stay below
Port/Adapter (rules 6-7) so TS files in adapter paths classify
correctly. See `specs/classifier-hardening.md` for the full
precedence table and conformance test plan.

### Tree-sitter grammars

Grammars are compiled in for Python, Go, Shell (bash), and Rust.
TypeScript has no grammar -- all TS/TSX files get empty
`top_level_kinds` and rely on the lang-based catch-all. Languages
without grammars (Markdown, TOML, YAML, JSON, Nushell, BAML) also
get empty kinds and classify by lang or path only.

### Ecosystem routing

`repos.json` is a JSON array of `{name, description, url}` objects.
`route()` applies fixed rules first (Discard->Discard,
TestHarness->Discard, Config->Archive, Script->keyword match), then
falls back to keyword scoring against repo names/descriptions.
Unmatched items suggest a new crate name from the file stem.

## Classifier Conformance Tests (planned)

The following 9 conformance tests are specified in
`specs/classifier-hardening.md` but **not yet implemented**.
When implemented in `tests/conformance_classifier.rs`, they must
pass before any classifier change is merged.

| ID  | Invariant                                                        |
| --- | ---------------------------------------------------------------- |
| C1  | Determinism -- same input, same output                           |
| C2  | All ItemKind variants are reachable (no dead categories)         |
| C3  | Path rules are case-insensitive                                  |
| C4  | Lang-only rules don't depend on path (no higher rule matching)   |
| C5  | Precedence pairs -- for each adjacent rule pair, higher one wins |
| C6  | Empty/degenerate inputs never panic                              |
| C7  | Unknown top_level_kinds strings fall through to Discard          |
| C8  | Regression: non-empty Rust with zero kinds != Glue (df6437a)     |
| C9  | Regression: TS under adapter paths = Adapter, not DomainLogic    |

If adding a new rule or reordering existing rules:

- Update C2 if a new ItemKind variant is introduced
- Add a new precedence pair to C5 for the new rule's position
- Add a regression test (C10+) if the change fixes a misclassification

## Test Fixtures

`tests/fixtures/` contains sample source files (`.py`, `.go`, `.sh`)
and `repos.json` for ecosystem routing tests. Parser tests use these
fixtures; ecosystem tests use `repos.json` via `fixture_repos()`.

## Specs

`specs/` contains feature specs for planned work:

- `discover.md` -- `cnbl discover` subcommand (ecosystem map generation)
- `classifier-hardening.md` -- regression tests and conformance suite
