---
name: cannibalizer
description: >
  Absorb a foreign repo into the Rust ecosystem. Full pipeline: scan,
  classify, plan routing, generate hexagonal stubs, and execute. Use
  when the user says "cannibalize", "absorb", "strip", or points at a
  non-Rust repo to harvest. Orchestrates the scan -> plan -> gen -> eat
  pipeline with approval gates between stages.
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

Full pipeline orchestrator. Walks through all four stages with the user
approving each transition.

## When to Use

- User points at a repo and says "cannibalize", "absorb", or "strip"
- User wants to retire a non-Rust project but preserve useful logic
- A Python/Go/Shell project has domain logic worth porting to Rust

## Pipeline

```
cnbl scan <repo> --output scan.jsonl --report
    |
    v  (user reviews scan report)
cnbl plan --input scan.jsonl --repo-map repos.json --dry-run
    |
    v  (user reviews routing decisions)
cnbl gen --input plan.jsonl --out-dir cnbl-output
    |
    v  (user reviews generated stubs)
cnbl eat --input plan.jsonl --scaffold-dir cnbl-output --dry-run
    |
    v  (user approves, re-run without --dry-run)
```

## Procedure

1. **Scan**: run `cnbl scan <repo> --output /tmp/cnbl-scan.jsonl --report`
   Show the report table to the user. Wait for approval.

2. **Plan**: run `cnbl plan --input /tmp/cnbl-scan.jsonl --dry-run`
   Show routing decisions. Let user override any routing before proceeding.
   When approved: `cnbl plan --input /tmp/cnbl-scan.jsonl > /tmp/cnbl-plan.jsonl`

3. **Generate**: run `cnbl gen --input /tmp/cnbl-plan.jsonl --out-dir /tmp/cnbl-output`
   Show generated stubs. Wait for approval.

4. **Execute**: run `cnbl eat --input /tmp/cnbl-plan.jsonl --scaffold-dir /tmp/cnbl-output --dry-run`
   Show what would happen. When approved, re-run without `--dry-run`.

5. **Report**: summarize what was absorbed, archived, and discarded.

## Ecosystem Awareness

Before the plan phase, load the ecosystem map:

```bash
cnbl discover --path ~/dev --output /tmp/repos.json
```

Or if discover is not yet implemented, use the existing map:

```bash
cat ~/dev/bazaar/repos.json
```

Load `references/ecosystem-map.md` for the canonical repo-to-domain
mapping when reviewing routing decisions.

## References

- `references/ecosystem-map.md` -- repo-to-domain routing table
- `references/hex-component-template.md` -- hexagonal stub structure
