# cnbl discover -- Ecosystem Discovery

## Summary

Add a `discover` subcommand that scans a directory for git repositories,
extracts metadata from each, and writes a `repos.json` file. This
replaces the hardcoded `~/dev/bazaar/repos.json` path with a
runtime-generated ecosystem map rooted at `$PWD`.

## Motivation

The `plan` command routes harvest items to ecosystem repos using a
static `repos.json` that must be maintained by hand or synced from an
external source. The discover command makes the ecosystem map
self-describing: point it at any directory of git repos and it builds
the map automatically.

## CLI Interface

```
cnbl discover [--path <DIR>] [--output <FILE>]
```

| Flag       | Default      | Description                          |
| ---------- | ------------ | ------------------------------------ |
| `--path`   | `$PWD`       | Root directory to scan for git repos |
| `--output` | `repos.json` | Output path, relative to `--path`    |

**Path resolution:** `--path` is resolved to an absolute path at
invocation time via `std::fs::canonicalize`. `--output` is resolved
relative to `--path` (not cwd) unless it is already absolute.

**Stdout/stderr contract:**

- JSON output goes only to the file specified by `--output`.
- Progress and diagnostics go to stderr (`eprintln!`).
- The command produces no stdout, keeping it composable in pipelines.

**Exit codes:**

- `0` -- success (all repos discovered, output written).
- `1` -- fatal I/O error (cannot read root dir, cannot write output).
- `0` with stderr warnings -- partial success (some repos skipped;
  see Failure Semantics below).

## Discovery Scope

The scanner examines **only immediate child directories** of `--path`.
It does not recurse into subdirectories of those children.

A directory is recognized as a repository if and only if it contains
a `.git` **directory** (not a `.git` file, which indicates a worktree
or submodule).

**Excluded entries:**

- Hidden directories (name starts with `.`)
- Symlinks (to avoid alias duplicates)
- Files (non-directories)
- Entries the process cannot read (permission errors logged to stderr,
  not fatal)

**Not supported (non-goals for v1):**

- Git worktrees (`.git` is a file pointer)
- Git submodules
- Bare repositories
- Nested repositories (repos inside repos)

## Output Schema

Sorted JSON array of objects, one per discovered repo. Output is sorted
lexicographically by `name`. The sort is performed by the writer before
serialization.

```json
[
  {
    "name": "doob",
    "description": "Modern, agent-first todo CLI built with Rust and SurrealDB",
    "url": "https://github.com/89jobrien/doob",
    "pushed_at": "2026-04-19T01:14:45Z",
    "created_at": "2026-02-21T05:27:42Z",
    "topics": [],
    "stars": null,
    "license": "MIT OR Apache-2.0",
    "latest_release": null,
    "homepage": null,
    "default_branch": "main"
  }
]
```

### Field Sources

All git commands run with `git -C <repo-dir>` to avoid mutating any
working tree. Metadata extraction is read-only.

| Field            | Source                                                    | On failure     |
| ---------------- | --------------------------------------------------------- | -------------- |
| `name`           | Directory name (leaf component only)                      | always present |
| `description`    | Cargo.toml > package.json > first line of README\* > null | `null`         |
| `url`            | `git remote get-url origin`                               | `""` (empty)   |
| `pushed_at`      | `git log -1 --format=%cI` (HEAD)                          | `null`         |
| `created_at`     | `git log --reverse --format=%cI -1` (root commit)         | `null`         |
| `topics`         | Always `[]` (no local source)                             | `[]`           |
| `stars`          | Always `null` (local-only)                                | `null`         |
| `license`        | Cargo.toml `license` > package.json `license`             | `null`         |
| `latest_release` | `git tag --sort=-v:refname` first line                    | `null`         |
| `homepage`       | Cargo.toml `homepage` > package.json `homepage`           | `null`         |
| `default_branch` | See derivation below                                      | `null`         |

\*README variants checked in order: `README.md`, `README`, `readme.md`,
`readme`. Case-sensitive match. First non-empty line after stripping
leading `#` markdown headers.

**`pushed_at` / `created_at` semantics:** These timestamps reflect the
commit history of whatever branch is currently checked out. They do NOT
attempt to query a specific branch. `git log -1 --format=%cI` returns
the committer date of HEAD. `git log --reverse --format=%cI -1` returns
the committer date of the root commit reachable from HEAD.

**`default_branch` derivation:**

1. `git symbolic-ref --short HEAD` -- works when HEAD points to a
   branch.
2. If that fails (detached HEAD), try `git config init.defaultBranch`.
3. If that also fails, yield `null`.

This reports the currently checked-out branch, not the remote's default
branch. The field name matches the GitHub API schema for compatibility
with existing `repos.json` consumers.

**`url` semantics:** Only the `origin` remote is queried. If no
`origin` remote exists, `url` is an empty string. Multiple remotes and
malformed URLs are not normalized -- the raw output of
`git remote get-url origin` is used as-is.

**Metadata parsing failures:** If `Cargo.toml` or `package.json` exists
but is malformed (invalid TOML/JSON), the field falls through to the
next source in the precedence chain. Parse errors are logged to stderr
but are not fatal.

## Duplicate Name Policy

Because discovery scans only immediate children of a single directory,
duplicate directory names are impossible by filesystem constraint. The
conformance tests assert this invariant. If a future discoverer scans
multiple roots, it must define a deduplication strategy.

## Failure Semantics

Discovery is **best-effort / partial-success**:

- If a recognized repo directory fails metadata extraction (git errors,
  unreadable Cargo.toml, etc.), that repo is **skipped** with a warning
  on stderr.
- The remaining repos are still written to the output file.
- The exit code is still `0` if the output file was written
  successfully.
- If the root directory itself is unreadable or the output file cannot
  be written, exit code is `1`.

## Architecture

### Domain Types (`src/ecosystem.rs`)

Extend the existing module:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    #[serde(default)]
    pub pushed_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub stars: Option<u64>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub latest_release: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub default_branch: Option<String>,
}
```

The `#[serde(default)]` annotations ensure that legacy `EcosystemRepo`
JSON (which only has `name`, `description`, `url`) deserializes into
`RepoInfo` with missing fields set to `None`/`[]`. Unknown fields in
input JSON are silently ignored (`#[serde(deny_unknown_fields)]` is NOT
set).

### Ports (traits)

```rust
/// Discovers repositories under a root directory.
pub trait RepoDiscoverer {
    fn discover(&self, root: &Path) -> Result<Vec<RepoInfo>, DiscoverError>;
}

/// Writes a set of discovered repos to a destination.
pub trait RepoWriter {
    fn write(&self, repos: &[RepoInfo], dest: &Path) -> Result<(), DiscoverError>;
}
```

### Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("git error in {repo}: {message}")]
    Git { repo: String, message: String },
    #[error("metadata parse error in {repo}/{file}: {message}")]
    MetadataParse {
        repo: String,
        file: String,
        message: String,
    },
}
```

`Git` and `MetadataParse` are non-fatal at the command level (repo is
skipped). `Io` and `Serialize` are fatal.

### Adapters

| Adapter             | File                             | Implements       |
| ------------------- | -------------------------------- | ---------------- |
| `GitRepoDiscoverer` | `src/adapters/git_discoverer.rs` | `RepoDiscoverer` |
| `JsonRepoWriter`    | `src/adapters/json_writer.rs`    | `RepoWriter`     |

**GitRepoDiscoverer** reads immediate children of `root` via
`std::fs::read_dir`. For each entry that is a non-hidden, non-symlink
directory containing a `.git/` directory, it extracts metadata using
the field source rules above. Repos that fail extraction are logged
to stderr and omitted from the result. The returned `Vec<RepoInfo>` is
sorted by `name`.

**JsonRepoWriter** serializes `Vec<RepoInfo>` as pretty-printed JSON
(`serde_json::to_writer_pretty`). Key ordering follows serde's default
(struct field declaration order). The writer assumes input is already
sorted.

### Command (`src/cmd/discover.rs`)

```rust
pub fn run(root: &Path, output: &Path) -> Result<()> {
    let discoverer = GitRepoDiscoverer;
    let writer = JsonRepoWriter;
    let repos = discoverer.discover(root)?;
    writer.write(&repos, output)?;
    eprintln!("discovered {} repos -> {}", repos.len(), output.display());
    Ok(())
}
```

### Wiring

- `cli.rs`: add `Discover { path, output }` variant
- `main.rs`: match on `Command::Discover`, resolve paths, call
  `cmd::discover::run`

## Testing Strategy

### Unit Tests

In `src/ecosystem.rs` and adapter modules:

- `RepoInfo` serde roundtrip (serialize -> deserialize = identity)
- `RepoInfo` with all optional fields `None` and `topics: []`
  roundtrips correctly
- Legacy `EcosystemRepo` JSON (3 fields) deserializes into `RepoInfo`
  with defaults
- `GitRepoDiscoverer` skips non-git directories (tempdir without
  `.git/`)
- `GitRepoDiscoverer` skips hidden directories (`.hidden/`)
- `GitRepoDiscoverer` skips symlinks
- `JsonRepoWriter` produces valid JSON array
- `JsonRepoWriter` output is pretty-printed (contains newlines)
- `JsonRepoWriter` output is sorted by `name`
- Empty directory yields empty `Vec<RepoInfo>` and valid `[]` JSON

### Integration Tests (`tests/integration_discover.rs`)

Against real tempdir fixtures with actual git repos:

- Create 3 tempdir repos with `git init`, varying Cargo.toml/README
  content. Run discover, assert all 3 appear with correct names.
- Repo with no remote yields empty `url` string.
- Repo with Cargo.toml description extracts it correctly.
- Repo with only README.md extracts first line as description.
- Mixed directory (some git repos, some plain dirs, some hidden,
  some symlinks) yields only the git repos.
- Output file is valid JSON parseable back to `Vec<RepoInfo>`.
- Repo with detached HEAD yields `default_branch: null` (or value
  from `init.defaultBranch` config).
- Repo whose default branch is `develop` (not `main`) reports
  `default_branch: "develop"`.
- Repo with malformed Cargo.toml still appears in output with
  `description: null`.
- Repos in output are sorted by name.

### Conformance Tests (`tests/conformance.rs`)

Generic test suite parameterized over `RepoDiscoverer`:

```rust
fn conformance_suite<D: RepoDiscoverer>(discoverer: &D, root: &Path) {
    let repos = discoverer.discover(root).unwrap();

    // C1: no duplicate names
    let names: HashSet<_> = repos.iter().map(|r| &r.name).collect();
    assert_eq!(names.len(), repos.len());

    // C2: every repo has a non-empty name
    assert!(repos.iter().all(|r| !r.name.is_empty()));

    // C3: name contains no path separators
    assert!(repos.iter().all(|r|
        !r.name.contains('/') && !r.name.contains('\\')
    ));

    // C4: pushed_at is ISO 8601 or None
    for r in &repos {
        if let Some(ref ts) = r.pushed_at {
            assert!(ts.contains('T'), "pushed_at must be ISO 8601");
        }
    }

    // C5: created_at is ISO 8601 or None
    for r in &repos {
        if let Some(ref ts) = r.created_at {
            assert!(ts.contains('T'), "created_at must be ISO 8601");
        }
    }

    // C6: default_branch is non-empty when present
    for r in &repos {
        if let Some(ref b) = r.default_branch {
            assert!(!b.is_empty());
        }
    }

    // C7: output is sorted by name
    let sorted: Vec<_> = {
        let mut names: Vec<_> =
            repos.iter().map(|r| &r.name).collect();
        names.sort();
        names
    };
    let actual: Vec<_> = repos.iter().map(|r| &r.name).collect();
    assert_eq!(actual, sorted);

    // C8: deterministic -- calling discover twice yields same result
    let repos2 = discoverer.discover(root).unwrap();
    assert_eq!(repos, repos2);
}
```

Any new `RepoDiscoverer` impl must pass this suite.

### Property Tests (`tests/properties.rs`)

Using `proptest`:

- **Roundtrip**: arbitrary `RepoInfo` -> JSON -> deserialize = original
- **No panics on empty fields**: `RepoInfo` with all `None`/empty
  fields serializes without panic
- **Name invariant**: `name` field never contains path separators
- **Idempotent write**: write same repos twice, file content identical

Proptest strategy for `RepoInfo`:

```rust
fn arb_repo_info() -> impl Strategy<Value = RepoInfo> {
    (
        "[a-z][a-z0-9_-]{0,30}",
        prop::option::of("[^\0]{0,100}"),
        "(https?://[a-z.]+/[a-z0-9/-]+)?",
        prop::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:]+Z"),
        prop::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:]+Z"),
        prop::collection::vec("[a-z-]{1,20}", 0..5),
        prop::option::of(0u64..10000),
        prop::option::of("[A-Z a-z0-9./-]{0,30}"),
        prop::option::of("v[0-9]+\\.[0-9]+\\.[0-9]+"),
        prop::option::of("https?://[a-z.]+"),
        prop::option::of("(main|master|develop)"),
    ).prop_map(|(name, description, url, pushed_at, created_at,
                 topics, stars, license, latest_release, homepage,
                 default_branch)| {
        RepoInfo {
            name, description, url, pushed_at, created_at,
            topics, stars, license, latest_release, homepage,
            default_branch,
        }
    })
}
```

### Fuzz Targets (`fuzz/fuzz_targets/repo_info_deser.rs`)

Using `cargo-fuzz` / `libfuzzer`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Vec<cnbl::RepoInfo>>(data);
    let _ = serde_json::from_slice::<cnbl::RepoInfo>(data);
});
```

Target: no panics, no UB. OOM and timeouts are acceptable corpus
limits.

## Dependencies

New:

- `thiserror` -- typed errors (replaces ad-hoc anyhow for discover;
  existing `anyhow` remains for other commands)
- `proptest` (dev-dependency)
- `libfuzzer-sys` (fuzz target only, in `fuzz/Cargo.toml`)
- `arbitrary` + `derive` (dev-dependency, for structured fuzzing)

## Migration

### Backward Compatibility

`RepoInfo` is a strict superset of `EcosystemRepo`. The three shared
fields (`name`, `description`, `url`) have identical types. All new
fields on `RepoInfo` have `#[serde(default)]`, so:

- Old `repos.json` files with only 3 fields per object deserialize
  into `RepoInfo` with defaults (`None`/`[]`).
- New `repos.json` files with all fields deserialize into the old
  `EcosystemRepo` type because serde ignores unknown fields by
  default.

### Steps

1. Replace `EcosystemRepo` with `RepoInfo` in `ecosystem.rs`. The
   existing `load_repos()` function signature changes from
   `Vec<EcosystemRepo>` to `Vec<RepoInfo>`. Callers (`route()`,
   `domain_match()`, `repo_decision()`) update field access -- all
   three used fields are unchanged.
2. `main.rs` `default_repo_map()`: change fallback from
   `~/dev/bazaar/repos.json` to `$PWD/repos.json`.
3. `SKILL.md` ecosystem awareness section: replace
   `open ~/dev/bazaar/repos.json` with `cnbl discover --path $PWD`.
4. Update `tests/fixtures/repos.json` to include both old-format
   (3-field) and new-format (all-field) entries so fixture tests
   exercise backward compatibility.

## Semantic Fingerprints (Phase 2)

### Motivation

The `plan` command routes items using keyword overlap between file path
components and repo name/description (`domain_match()` in
`ecosystem.rs`). This fails when names don't match semantics -- e.g.
`src/auth_service.py` won't match a repo named `sanctum` even though
sanctum handles auth. Embeddings solve this by comparing what code
_does_, not what it's _named_.

### Concept

Each discovered repo gets a **semantic fingerprint**: a fixed-length
vector that captures what the repo's code does. At plan time, harvested
items are embedded and routed by cosine similarity against repo
fingerprints instead of keyword matching.

### How It Works

1. **Embed**: for each repo, use tree-sitter to chunk source files
   into functions/structs/traits (reusing the existing
   `scanner/parser.rs` infrastructure). Embed each chunk via
   nomic-embed-text (Ollama, local, no API key).
2. **Aggregate**: compute a centroid vector per repo (mean of all
   chunk embeddings). This is the repo's semantic fingerprint.
3. **Store**: add an `embedding` field to `RepoInfo`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoInfo {
    // ... existing fields ...

    /// Semantic fingerprint -- centroid of source code embeddings.
    /// 768-dim vector (nomic-embed-text). `None` if embeddings were
    /// not computed (e.g. `--no-embed` flag or empty repo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}
```

4. **Route**: in `plan`, replace or augment `domain_match()` with
   cosine similarity between the harvested item's embedding and each
   repo's fingerprint. Fall back to keyword matching when embeddings
   are unavailable.

### CLI Extension

```
cnbl discover [--path <DIR>] [--output <FILE>] [--embed] [--embed-model <MODEL>]
```

| Flag            | Default            | Description                   |
| --------------- | ------------------ | ----------------------------- |
| `--embed`       | off                | Compute semantic fingerprints |
| `--embed-model` | `nomic-embed-text` | Ollama model for embeddings   |

When `--embed` is not set, `embedding` fields are `null` and
discovery behaves exactly as phase 1. This keeps the fast path fast
and avoids requiring Ollama for basic discovery.

### Embedding Provider Port

```rust
/// Embeds text chunks into vectors.
pub trait Embedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dimensions(&self) -> usize;
}
```

Adapters:

- `OllamaEmbedder` -- calls local Ollama API (`/api/embed`)
- `MockEmbedder` -- returns deterministic vectors for testing

### Chunking Strategy

Reuse tree-sitter parsing from `scanner/parser.rs`. For each source
file in the repo:

1. Parse with tree-sitter to get top-level nodes.
2. Extract text of each function/struct/trait/class/type node.
3. Skip nodes smaller than 50 chars (noise) or larger than 4096
   chars (truncate to model context window).
4. Embed each chunk as a separate text.
5. Repo centroid = mean of all chunk vectors, L2-normalized.

For repos with no parseable source (docs-only, empty), `embedding`
is `null`.

### Similarity Function

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
```

### Integration with Plan

`domain_match()` gains a new code path:

```rust
pub fn domain_match(
    item: &HarvestItem,
    item_embedding: Option<&[f32]>,
    repos: &[RepoInfo],
) -> Option<&RepoInfo> {
    // If both item and repos have embeddings, use cosine similarity.
    if let Some(item_emb) = item_embedding {
        let best = repos.iter()
            .filter_map(|r| {
                r.embedding.as_ref()
                    .map(|re| (r, cosine_similarity(item_emb, re)))
            })
            .filter(|(_, sim)| *sim > 0.5)  // threshold
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        if let Some((repo, _)) = best {
            return Some(repo);
        }
    }
    // Fall back to keyword matching.
    keyword_match(keywords, repos)
}
```

The similarity threshold (0.5) is configurable. Items below threshold
fall through to keyword matching, then to "suggest new crate".

### Prior Art

- **codegraph-rust** -- Rust + tree-sitter + embeddings + SurrealDB.
  Same stack. Solves embed+index+search for MCP code intelligence.
- **code2vec** -- AST path decomposition into fixed-length vectors.
  Foundational research on code embeddings.
- **opencode-codebase-index** -- tree-sitter chunking + uSearch +
  BM25 hybrid search. Demonstrates the chunk-embed-search pipeline.
- **MediaWiki Code2Code Search** (April 2026) -- cross-language code
  similarity via vector representations.

### Testing (Phase 2 additions)

**Unit:**

- `cosine_similarity` returns 1.0 for identical vectors
- `cosine_similarity` returns 0.0 for orthogonal vectors
- `cosine_similarity` handles zero-length vectors without panic
- Centroid of N identical vectors equals that vector
- `OllamaEmbedder` returns 768-dim vectors

**Integration:**

- Discover with `--embed` against tempdir repos produces non-null
  `embedding` fields with correct dimensionality
- Discover without `--embed` produces null `embedding` fields
- Plan with embeddings routes `src/auth_service.py` to the repo
  whose code is semantically closest to auth logic

**Conformance (extended):**

- C9: when `embedding` is present, it has exactly
  `embedder.dimensions()` elements
- C10: when `embedding` is present, it is L2-normalized
  (length ~= 1.0)

**Property:**

- Roundtrip: `RepoInfo` with `embedding: Some(vec)` serializes and
  deserializes to the same vector within f32 tolerance
- Cosine similarity is symmetric: `sim(a, b) == sim(b, a)`
- Cosine similarity is bounded: `-1.0 <= sim <= 1.0`

**Fuzz:**

- Extend `repo_info_deser` fuzz target to cover `embedding` field
  (arbitrary-length float arrays, NaN, infinity, empty arrays)

### Dependencies (Phase 2)

- `reqwest` (or `ureq`) -- HTTP client for Ollama API
- No new heavy dependencies; embedding math is hand-rolled (no
  ndarray/nalgebra needed for centroid + cosine)

### Phase 2 Scope Boundary

Phase 2 is additive and backward-compatible:

- `repos.json` without `embedding` fields works identically to
  phase 1 (serde default = `None`).
- `plan` without embeddings uses keyword matching (existing
  behavior).
- `--embed` is opt-in; Ollama is not required for basic usage.
- The `Embedder` trait allows swapping models later (e.g.
  Jina v2 Base Code for 768-dim code-specific embeddings).

## Non-Goals

- No GitHub API calls. Discovery is local-only.
- No recursive scanning (only immediate children of `--path`).
- No caching or incremental updates (always full scan).
- No domain routing in this command -- that stays in `plan`.
- No worktree, submodule, or bare repo detection (v1).
- No remote default-branch resolution (uses local HEAD).
- No vector database (phase 2 stores embeddings inline in
  repos.json; a separate index is out of scope).
