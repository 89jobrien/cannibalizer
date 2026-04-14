# cannibalizer

Claude Code plugin — absorbs foreign repos into the Rust ecosystem.

Scans a source repo, classifies every file by kind (domain type, port, adapter,
script, spec, config), then routes each item to the right destination: ported to
a hexagonal Rust component, absorbed into an existing repo, converted to a skill,
or archived to the SSD vault.

## Install

Add to your Claude Code plugins via bazaar:

```json
{
  "name": "cannibalizer",
  "source": { "source": "github", "repo": "89jobrien/cannibalizer" }
}
```

## Usage

Point it at a repo:

```
cannibalize ~/ai-vault
cannibalize ~/agent-os
```

The skill will scan, classify, generate a harvest plan, and execute with your
approval at each step.

## Scripts

`skills/cannibalizer/scripts/scan.nu` — Nushell scanner that emits a JSONL
stream of classified files from the target repo.

## License

MIT OR Apache-2.0
