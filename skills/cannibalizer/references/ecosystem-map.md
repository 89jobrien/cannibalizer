# Ecosystem Map

Maps domain concerns to the correct destination repo. Use this during the
classify phase to route absorbed items.

| domain | repo | notes |
|--------|------|-------|
| todo / task management | `doob` | SurrealDB-backed, hexagonal |
| log redaction / PII scrubbing | `obfsck` | `redact` binary |
| formatter dispatch / pre-commit hooks | `fmtx` | extension → command mapping |
| Claude Code hooks / command blocking | `coursers` | PreToolUse/PostToolUse |
| LLM agent loop / REPL | `looprs` | multi-provider, extensible |
| session log parsing / dashboards | `peeprs` | Axum, JSONL ingestion |
| dotfiles / machine bootstrap | `notfiles` | replaces GNU Stow |
| data harvesting → JSONL | `harvestrs` | multi-source sync |
| MCP / OpenAPI / GraphQL → CLI | `mcpipe` | backend adapters |
| semantic Rust linting | `rascal` | tree-sitter, corpus scoring |
| Claude Code plugin distribution | `bazaar` | marketplace registry |
| dev workflow / CI / git safety | `atelier` | skills + agents |
| 1Password / direnv session mgmt | `sanctum` | Claude Code plugin |
| parallel TDD orchestration | `orca-strait` | Rust workspace agent |

## Routing Rules

- If a component crosses two domains, prefer the repo whose port it satisfies
- If no repo fits, propose a new crate under the most related workspace
- Never route infrastructure code into a domain layer
