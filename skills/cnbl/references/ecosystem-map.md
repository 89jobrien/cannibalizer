# Ecosystem Map

Maps domain concerns to the correct destination repo. Use this during
the classify phase to route absorbed items.

## doob

**Domain:** todo / task management / project tracking

SurrealDB-backed task tracker with GitHub sync, handoff workflows,
and CLI. Hexagonal architecture (ports/adapters). Owns todo CRUD,
status transitions, due dates, and priority triage.

## obfsck

**Domain:** log redaction / PII scrubbing / secret detection

Pre-commit secret-detection and redaction CLI. Provides the `redact`
binary for scanning staged files against pattern rules. Owns
allowlists, severity levels, and JSONL audit output.

## fmtx

**Domain:** formatter dispatch / pre-commit hooks / linting orchestration

Maps file extensions to formatter commands and orchestrates
pre-commit formatting. Owns extension-to-command mappings and
format-on-save dispatch logic.

## coursers

**Domain:** Claude Code hooks / command blocking / tool-use guards

Claude Code PreToolUse/PostToolUse hook scripts. Blocks anti-pattern
CLI commands, tracks failure rates, and rewrites commands through
RTK. Owns hook rule definitions and course-correct logic.

## looprs

**Domain:** LLM agent loop / REPL / multi-provider chat

Multi-provider LLM agent loop with extensible REPL interface.
Supports streaming, tool use, and provider switching. Owns the core
chat loop, provider adapters, and conversation state.

## peeprs

**Domain:** session log parsing / dashboards / observability

Axum-based web service for ingesting and visualizing JSONL session
logs. Owns log parsing, dashboard rendering, and session analytics.

## notfiles

**Domain:** dotfiles / machine bootstrap / shell config

Declarative machine bootstrap replacing GNU Stow. Owns symlink
management, shell configuration (nu/zsh), SSH setup, and
system-level dotfile distribution.

## harvestrs

**Domain:** data harvesting / sync pipelines / JSONL ingestion

Multi-source data sync to JSONL format. Owns source adapters (APIs,
files, databases), transform pipelines, and incremental sync state.

## mcpipe

**Domain:** MCP servers / OpenAPI proxying / GraphQL-to-CLI bridging

Adapter layer that exposes MCP servers, OpenAPI specs, and GraphQL
endpoints as CLI tools. Owns backend adapter generation, protocol
translation, and tool registration.

## rascal

**Domain:** semantic Rust linting / code quality scoring

Tree-sitter-based semantic linter for Rust. Scores code against a
corpus of patterns and conventions. Owns lint rule definitions, AST
queries, and quality metrics.

## bazaar

**Domain:** Claude Code plugin distribution / marketplace registry

Plugin marketplace for Claude Code. Hosts plugin metadata, handles
installation and version resolution. Owns the registry index, plugin
manifests, and `repos.json` ecosystem catalog.

## atelier

**Domain:** dev workflow / CI pipelines / git safety / agent skills

Claude Code plugin providing handoff, herald, sentinel, forge, and
other agent skills. Owns CI-assist workflows, git guard checks,
branch cleanup, and cross-project orchestration.

## sanctum

**Domain:** 1Password / direnv / session secret management

Claude Code plugin for 1Password secret resolution. Owns `op://`
URI resolution, direnv integration, and session-scoped secret
injection.

## orca-strait

**Domain:** parallel TDD orchestration / multi-crate agent dispatch

Rust workspace agent that orchestrates parallel TDD sub-agents
across crates. Reads GitHub issues and HANDOFF state, dispatches
worktree-isolated agents, and merges results.

## Routing Rules

- If a component crosses two domains, prefer the repo whose port it
  satisfies (e.g., a secret-aware formatter hook belongs in `fmtx`,
  not `obfsck`, because it satisfies the formatter dispatch port)
- If no repo fits, propose a new crate under the most related
  workspace
- Never route infrastructure code (CI scripts, Dockerfiles, deploy
  manifests) into a domain layer -- archive or discard instead
- Prefer absorbing into an existing repo over creating a new one;
  only propose a new crate when the domain is genuinely unserved
- When keyword scoring is ambiguous between two repos, check which
  repo's `ports/` traits the component would implement
