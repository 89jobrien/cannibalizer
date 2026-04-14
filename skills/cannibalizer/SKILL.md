---
name: cannibalizer
description: >
  Absorb a foreign repo into the Rust ecosystem. Scans the source for valuable
  components — specs, scripts, domain logic, types, patterns — classifies each
  item as absorb/port/archive/discard, then generates hexagonal Rust components
  or routes items to the correct existing repo. Use when the user says
  "cannibalize", "absorb", "strip", or points at a non-Rust repo to harvest.
model: sonnet
effort: high
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - Glob
  - Grep
  - Agent
---

# cannibalizer

Scan a foreign repo, extract what's worth keeping, and absorb it into the Rust
ecosystem as hexagonal components, skill updates, or archived artifacts.

## When to Use

- User points at a repo and says "cannibalize", "absorb", or "strip"
- User wants to retire a non-Rust project but preserve useful logic
- A Python/Go/Shell project has domain logic worth porting to Rust

## Ecosystem Awareness

Before classifying anything, load the current ecosystem state:

```nu
open ~/dev/bazaar/repos.json | select name description url
```

Match extracted components against existing repos. Prefer absorbing into an
existing repo over creating a new one.

Load `references/ecosystem-map.md` for the canonical repo-to-domain mapping.

## Scan Phase

Run the scanner on the target repo:

```nu
nu scripts/scan.nu <repo-path>
```

The scanner emits a JSONL stream of items:

```json
{"path": "src/domain/user.py", "kind": "domain-type", "summary": "User entity with auth fields"}
{"path": "scripts/harvest.sh", "kind": "script", "summary": "Syncs logs to JSONL"}
{"path": "specs/api.md", "kind": "spec", "summary": "REST API design doc"}
{"path": "notebooks/analysis.ipynb", "kind": "discard", "summary": "One-off analysis"}
```

Kinds: `domain-type`, `port`, `adapter`, `script`, `spec`, `skill`, `config`, `discard`

## Classify Phase

For each item, decide:

| disposition | when |
|-------------|------|
| `absorb` | Logic fits an existing repo's domain — route it there |
| `port` | Worth rewriting as a Rust hexagonal component |
| `new-skill` | Reusable agent workflow — add to skills |
| `archive` | Useful reference but not active — copy to SSD vault |
| `discard` | One-off, stale, or already covered elsewhere |

## Generate Phase

For items classified as `port`, generate a hexagonal Rust component.

Load `references/hex-component-template.md` for the canonical structure.

Follow the writing-solid-rust skill conventions:
- Domain type in `src/domain/`
- Port trait in `src/ports/`
- Adapter(s) in `src/adapters/`
- No infrastructure in domain layer

## Archive Phase

For items classified as `archive`, use the bazaar archive script:

```bash
~/dev/bazaar/scripts/archive-project.sh <name>
```

Update `~/Documents/Obsidian Vault/archived-projects.md` with a note on what
was harvested before archiving.

## Output

Produce a harvest report:

```
## Harvest Report — <repo-name>

### Absorbed
- `src/domain/user.py` → `doob/src/domain/user.rs` (port)

### New Skills
- `scripts/harvest.sh` → `atelier/skills/harvest/SKILL.md`

### Archived
- `notebooks/` → SSD vault/cannibal-artifacts/

### Discarded
- `*.ipynb` — one-off analysis, no reuse value
```

Commit each change to its destination repo before moving on to the next item.
