#!/usr/bin/env nu
# Scan a repo and classify files into cannibalizer item kinds.
# Usage: nu scan.nu <repo-path>

def main [repo: string] {
  let extensions = {
    domain: [rs py go ts]
    script: [sh nu bash py]
    spec:   [md txt]
    config: [toml yaml json]
    notebook: [ipynb]
  }

  let ignore_globs = [
    "*/.git/*"
    "*/node_modules/*"
    "*/target/*"
    "*/__pycache__/*"
    "*/dist/*"
    "*/.venv/*"
  ]

  glob $"($repo)/**/*"
  | where { |f| ($f | path type) == "file" }
  | where { |f|
      not ($ignore_globs | any { |pat| ($f | str contains $pat) })
  }
  | each { |f|
      let ext = ($f | path parse | get extension? | default "")
      let rel = ($f | str replace $"($repo)/" "")
      let kind = (classify $rel $ext)
      { path: $rel, kind: $kind, ext: $ext }
  }
  | where kind != "skip"
  | to jsonl
}

def classify [rel: string, ext: string] {
  # Notebooks are almost always discards
  if $ext == "ipynb" { return "discard" }

  # Lock files, build artifacts
  if ($rel =~ '(Cargo\.lock|package-lock\.json|uv\.lock|\.DS_Store)') {
    return "skip"
  }

  # Specs and design docs
  if ($rel =~ '(spec|design|plan|doc|spec)' and $ext == "md") {
    return "spec"
  }

  # Skills
  if ($rel =~ 'SKILL\.md$') { return "skill" }

  # Scripts
  if $ext in [sh nu bash] { return "script" }

  # Domain types (Python/Go/TS that could be ported to Rust)
  if ($rel =~ '(domain|model|entity|type)' and $ext in [py go ts]) {
    return "domain-type"
  }

  # Ports / interfaces
  if ($rel =~ '(port|interface|trait|protocol)' and $ext in [py go ts rs]) {
    return "port"
  }

  # Adapters / integrations
  if ($rel =~ '(adapter|integration|provider|backend)' and $ext in [py go ts rs]) {
    return "adapter"
  }

  # Config
  if $ext in [toml yaml json] { return "config" }

  # Markdown that isn't a spec
  if $ext == "md" { return "spec" }

  # Source files not matched above
  if $ext in [py go ts rs] { return "domain-type" }

  "skip"
}
