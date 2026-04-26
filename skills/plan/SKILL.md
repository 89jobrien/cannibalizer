---
name: plan
description: >
  Run cnbl plan to route classified items to ecosystem repos. Takes
  scan JSONL as input, matches items against repos.json, and outputs
  routing decisions. Use when the user wants to "plan migration",
  "route items", or "decide where files go".
model: haiku
effort: low
allowed-tools:
  - Bash
  - Read
---

# plan

Route scan output to ecosystem destinations.

## Usage

```bash
# Dry-run to preview decisions
cnbl plan --input /tmp/cnbl-scan.jsonl --dry-run

# Full output
cnbl plan --input /tmp/cnbl-scan.jsonl --repo-map repos.json > /tmp/cnbl-plan.jsonl
```

## Routing Rules

Fixed-destination rules (checked first):

- `Discard` kind -> Discard
- `TestHarness` kind -> Discard (tests stay with source)
- `Config` kind -> Archive
- `Script` kind -> keyword match (harvest->harvestrs, hook->coursers,
  fmt->fmtx) or Archive

Then keyword scoring against `repos.json`:

- Path components matched against repo name + description
- Highest-scoring repo wins
- Zero-score items suggest a new crate from the file stem

## Output

JSONL where each line is a `RouteDecision`:

```json
{
  "item": {
    "rel_path": "src/model.py",
    "lang": "python",
    "kind": "domain_logic",
    "size_bytes": 1234,
    "notes": null
  },
  "destination": {
    "type": "existing_repo",
    "name": "doob",
    "url": "https://..."
  },
  "rationale": "keyword match on repo 'doob'"
}
```

## Destinations

| Type          | Meaning                                    |
| ------------- | ------------------------------------------ |
| existing_repo | Route to a known repo in the ecosystem map |
| new_crate     | Suggest creating a new Rust crate          |
| archive       | Copy to vault for reference                |
| discard       | Drop entirely                              |
