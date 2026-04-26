# cannibalizer

Absorbs foreign repos into the Rust ecosystem.

Scans a source repo, classifies every file by kind (domain logic, port, adapter,
script, spec, config), then routes each item to the right destination: ported to
a hexagonal Rust component, absorbed into an existing repo, converted to a skill,
or archived to the SSD vault.

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# binary at target/release/cnbl
```

## CLI Pipeline

`cnbl` has a 4-stage pipeline. Each stage reads JSONL from the previous stage
(stdin or `--input FILE`):

```
scan  -->  plan  -->  gen  -->  eat
```

| Command | Input          | Output          | Side effects             |
| ------- | -------------- | --------------- | ------------------------ |
| `scan`  | directory path | JSONL to stdout | none                     |
| `plan`  | scan JSONL     | JSONL to stdout | none                     |
| `gen`   | plan JSONL     | stub files      | writes to `cnbl-output/` |
| `eat`   | plan JSONL     | nothing         | copies stubs, archives   |

### Examples

```bash
# Scan a repo and save classified output
cnbl scan ~/some-repo --output scan.jsonl

# Generate a routing plan (dry-run shows decisions without writing)
cnbl plan --input scan.jsonl --repo-map repos.json --dry-run

# Generate hexagonal Rust stubs from the plan
cnbl gen --input plan.jsonl --out-dir cnbl-output

# Apply: copy stubs into destination repos, archive originals
cnbl eat --input plan.jsonl --source-repo some-repo --dry-run
```

## Claude Code Plugin

cannibalizer is also packaged as a Claude Code plugin with skills that wrap the
CLI stages. Install via bazaar:

```json
{
  "name": "cannibalizer",
  "source": { "source": "github", "repo": "89jobrien/cannibalizer" }
}
```

Skills in `skills/` invoke the `cnbl` binary and present results interactively.
The `cannibalizer` skill in `skills/cnbl/SKILL.md` orchestrates the full
scan-plan-gen-eat pipeline with approval gates at each step.

## Language Support

| Language   | Parser         | Classification |
| ---------- | -------------- | -------------- |
| Rust       | tree-sitter    | structural     |
| Python     | tree-sitter    | structural     |
| Go         | tree-sitter    | structural     |
| Shell/Bash | tree-sitter    | structural     |
| TypeScript | none (planned) | path + lang    |
| Nushell    | none           | lang only      |
| Markdown   | none           | lang only      |
| TOML/YAML  | none           | lang only      |
| JSON/BAML  | none           | lang only      |

Languages without a tree-sitter grammar classify by path patterns and language
detection only. TypeScript support is intentionally conservative -- all TS/TSX
files are promoted to DomainLogic unless a higher-priority path rule (adapter,
port, entrypoint) matches first.

## License

MIT OR Apache-2.0
