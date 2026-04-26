---
name: gen
description: >
  Run cnbl gen to create hexagonal Rust stubs from a migration plan.
  Generates domain types, port traits, and adapter skeletons. Use when
  the user wants to "generate stubs", "scaffold components", or
  "create Rust skeletons".
model: haiku
effort: low
allowed-tools:
  - Bash
  - Read
  - Glob
---

# gen

Generate hexagonal Rust component stubs from plan output.

## Usage

```bash
cnbl gen --input /tmp/cnbl-plan.jsonl --out-dir /tmp/cnbl-output

# Overwrite existing stubs
cnbl gen --input /tmp/cnbl-plan.jsonl --out-dir /tmp/cnbl-output --force
```

## What Gets Generated

| Item Kind   | Generated Files                                   |
| ----------- | ------------------------------------------------- |
| DomainLogic | `<repo>/src/domain/<stem>.rs` -- struct stub      |
| Port        | `<repo>/src/ports/<stem>.rs` -- trait stub        |
|             | `<repo>/src/adapters/<stem>_stub.rs` -- impl stub |
| Adapter     | `<repo>/src/adapters/<stem>.rs` -- struct stub    |
| Other kinds | Not scaffolded                                    |

Each stub file includes:

- Source path comment (where it came from)
- Rationale comment (why it was routed here)
- TODO markers for manual porting

A `migration-notes.md` is written per destination repo listing all
ported items.

## Conflict Detection

Without `--force`, gen aborts if any output file already exists. The
error message lists all conflicting paths.

## Hexagonal Structure

Stubs follow the hex-component-template:

- Domain types: pure structs, no I/O deps
- Port traits: interface only, no implementation
- Adapters: implement one port, infrastructure allowed

See `skills/cnbl/references/hex-component-template.md` for the
canonical template.
