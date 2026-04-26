---
name: eat
description: >
  Run cnbl eat to execute a migration plan -- copies generated stubs
  into destination repos and archives originals to the vault. Use when
  the user wants to "execute the plan", "absorb the files", "do the
  migration", or "eat it".
model: haiku
effort: low
allowed-tools:
  - Bash
  - Read
---

# eat

Execute a migration plan by copying stubs and archiving originals.

## Usage

```bash
# Preview what would happen
cnbl eat --input /tmp/cnbl-plan.jsonl \
  --scaffold-dir /tmp/cnbl-output \
  --source-repo my-repo \
  --dry-run

# Execute for real
cnbl eat --input /tmp/cnbl-plan.jsonl \
  --scaffold-dir /tmp/cnbl-output \
  --source-repo my-repo
```

## Defaults

| Flag             | Default                                     |
| ---------------- | ------------------------------------------- |
| `--scaffold-dir` | `./cnbl-output`                             |
| `--repo-root`    | `~/dev`                                     |
| `--vault-dir`    | `/Volumes/Extreme SSD/vault/cnbl-artifacts` |
| `--source-repo`  | `unknown`                                   |

## Actions

| Destination  | Action                                        |
| ------------ | --------------------------------------------- |
| ExistingRepo | Copy scaffold dir into `<repo-root>/<name>/`  |
| NewCrate     | Copy scaffold dir into `<repo-root>/<name>/`  |
| Archive      | Copy source file into `<vault-dir>/<source>/` |
| Discard      | No action (logged in dry-run)                 |

## Safety

- Always run with `--dry-run` first and show the user the output
- Skips repos that don't exist at `<repo-root>/<name>/`
- Reports errors but continues processing remaining items
- Does not delete source files -- originals are preserved
