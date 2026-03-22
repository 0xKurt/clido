# Feature Plan: `.gitignore`-aware Repository Index

**Status:** Planned
**Target release:** V2
**Crates affected:** `clido-index`
**Docs affected:** `docs/guide/index-search.md`, `docs/reference/config.md`

---

## Problem Statement

`RepoIndex::build()` in `crates/clido-index/src/lib.rs` currently walks the file system without consulting `.gitignore`, `.dockerignore`, or any other ignore rules. This means every index build includes:

- `node_modules/` — typically 50,000–200,000 files in a JavaScript project.
- `target/` — Rust build artifacts, often gigabytes in size with thousands of `.d`, `.rlib`, and `.rmeta` files.
- `.git/` — the git object store, pack files, and index; entirely irrelevant to code search.
- `dist/`, `build/`, `.next/`, `__pycache__/` — generated output directories.
- Lock files, minified JS, vendored code.

The consequences are severe:

1. **Index build time**: walking `node_modules` alone can take 30–60 seconds on a cold file system. On a monorepo with several packages this multiplies.
2. **SQLite database size**: indexing 100k+ tiny files bloats the SQLite database, slowing every subsequent query.
3. **Search quality**: symbol and text search returns thousands of matches from vendored or generated code, burying real results. The user has to mentally filter noise that a proper ignore-aware walker would never have included.
4. **Storage cost**: the index can balloon to hundreds of megabytes for a project that `git status` considers to have a few hundred source files.

### Why this was not caught earlier

The `Glob` and `Grep` tools already use the `ignore` crate (`ignore::WalkBuilder`) correctly. When the agent searches files, it correctly skips ignored directories. But `RepoIndex::build()` uses a different traversal (currently `walkdir` or `std::fs::read_dir`) that has no concept of ignore files. The inconsistency means `Grep` finds nothing in `node_modules`, but `RepoIndex` has indexed all of it — giving users a false sense that the index is comprehensive when it is actually contaminated.

---

## Competitive Analysis

| Tool | Ignore-aware indexing | Custom ignore file | Config-driven exclusions |
|---|---|---|---|
| Ripgrep (`rg`) | Yes (uses `ignore` crate) | `.ignore` files | `--glob` patterns |
| VS Code (file watcher) | Yes (respects .gitignore) | `.vscodeignore` | `files.exclude` setting |
| GitHub Copilot workspace index | Yes | No | No |
| Clido `Glob`/`Grep` tools | Yes (uses `ignore` crate) | No | No |
| Clido `RepoIndex` (current) | **No** | No | No |
| Clido `RepoIndex` (after this feature) | Yes | `.clido-ignore` | `[index] exclude_patterns` |

Our improvement over ripgrep and VS Code: the `.clido-ignore` file allows project-specific exclusions that should not be committed to the repo (`.gitignore` is committed and shared). This is analogous to `.git/info/exclude` but lives at the project root for discoverability. Additionally, `[index] exclude_patterns` in `config.toml` provides user-wide exclusions for patterns the user always wants to skip regardless of project (e.g., `"*.min.js"`, `"*.snap"`).

---

## Design Decisions and Rationale

### Use `ignore::WalkBuilder` instead of patching `walkdir`

The `ignore` crate (authored by Andrew Gallant, the ripgrep author) is the de facto standard for respecting `.gitignore` semantics in Rust. It handles:

- Nested `.gitignore` files (monorepo case).
- `.git/info/exclude`.
- Global gitignore (`core.excludesFile` in git config).
- `.ignore` files (ripgrep-specific additional ignores).
- Case-insensitive matching on macOS/Windows.

Writing a correct `.gitignore` parser from scratch is non-trivial. Using `ignore::WalkBuilder` is three lines of code and handles all edge cases. Since `clido-tools` already depends on `ignore`, adding it to `clido-index` adds no new dependency to the workspace.

### `.clido-ignore` as a separate file from `.gitignore`

`.gitignore` is committed and shared across the team. A `.clido-ignore` file at the project root is gitignore-syntax but local to the user's clido configuration. This allows excluding files that should exist in the repo (e.g., large test fixtures, `.env.example`) from the index without changing git behavior. `.clido-ignore` should be added to `.gitignore` in projects that use it, or users can add it to `.git/info/exclude` themselves.

`ignore::WalkBuilder` supports adding custom ignore files via `.add_custom_ignore_filename()`. This makes `.clido-ignore` support a one-line addition.

### `--include-ignored` as an escape hatch, not the default

Some use cases genuinely need to index build artifacts — for example, indexing compiled `.d.ts` declaration files in a TypeScript project to provide accurate type information, or indexing a vendored dependency that has local patches. The `--include-ignored` flag disables all ignore rules for a single index build. It is not stored in the database; it only affects the current build invocation. Running `clido index build` (without the flag) always respects ignore rules.

### `exclude_patterns` applies after gitignore, not instead of it

The `exclude_patterns` config key adds additional glob patterns on top of `.gitignore`. If a file is already excluded by `.gitignore`, these patterns have no effect. If a file is tracked by git but should be excluded from the index (e.g., large binary assets tracked in git-lfs), `exclude_patterns` is the right tool.

### Stats output is informational, not blocking

`clido index build` prints a summary line after completion: `Indexed 1,247 files. Skipped 84,312 files (gitignore rules).` This helps users confirm the feature is working and understand the scale of what was excluded. Stats are computed as a simple counter during the walk, not by re-walking.

---

## Implementation Steps (ordered)

### Step 1: Add `ignore` crate to `clido-index/Cargo.toml`

```toml
[dependencies]
ignore = "0.4"
```

Verify that the version used in `clido-tools/Cargo.toml` is the same to avoid duplicate compilation.

### Step 2: Replace the traversal in `RepoIndex::build()`

Current traversal (in `crates/clido-index/src/lib.rs`) uses `walkdir::WalkDir` or `std::fs::read_dir`. Replace with:

```rust
use ignore::WalkBuilder;

let walker = WalkBuilder::new(&self.root)
    .hidden(false)           // include dotfiles that are not ignored
    .git_ignore(true)        // respect .gitignore
    .git_global(true)        // respect global gitignore
    .git_exclude(true)       // respect .git/info/exclude
    .add_custom_ignore_filename(".clido-ignore")
    .build();

let mut indexed = 0u64;
let mut skipped = 0u64;

for result in walker {
    match result {
        Ok(entry) => {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                // apply exclude_patterns from config
                if self.matches_exclude_patterns(entry.path()) {
                    skipped += 1;
                    continue;
                }
                self.index_file(entry.path(), &mut tx)?;
                indexed += 1;
            }
        }
        Err(e) => {
            // log warning, do not abort
            tracing::warn!("Index walk error: {}", e);
            skipped += 1;
        }
    }
}
```

### Step 3: Implement `matches_exclude_patterns()`

```rust
fn matches_exclude_patterns(&self, path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    for pattern in &self.config.index.exclude_patterns {
        if glob::Pattern::new(pattern)
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}
```

Use the `glob` crate (already in workspace via `clido-tools`). Pre-compile patterns at index construction time, not per-file, for performance.

### Step 4: Add `IndexConfig` to `AgentConfig`

In `crates/clido-core/src/lib.rs`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct IndexConfig {
    pub exclude_patterns: Vec<String>,
    pub include_ignored: bool,
}
```

In `AgentConfig`:

```rust
pub struct AgentConfig {
    // ... existing fields
    pub index: IndexConfig,
}
```

Load in `config_loader.rs` under `[index]` table.

### Step 5: Wire `--include-ignored` flag

In `crates/clido-cli/src/cli.rs`, add to the `IndexBuild` subcommand args:

```rust
/// Bypass .gitignore and index all files including build artifacts.
#[arg(long)]
include_ignored: bool,
```

Pass through to `RepoIndex::build_with_options(root, options)`. When `include_ignored = true`, construct the walker with `.git_ignore(false).git_global(false).git_exclude(false)` and skip the `.clido-ignore` custom filename.

### Step 6: Print stats after build

After the walk loop completes, emit:

```
Indexed 1,247 files. Skipped 84,312 files (ignore rules).
```

If `--include-ignored` was passed:

```
Indexed 85,559 files (ignore rules bypassed).
```

### Step 7: Tests (see Test Plan below)

### Step 8: Documentation updates

---

## Configuration Schema

In `config.toml`:

```toml
[index]
# Additional glob patterns to exclude from the index, applied on top of .gitignore.
# These supplement (do not replace) .gitignore rules.
# Example patterns: "*.min.js", "*.snap", "fixtures/large/**"
exclude_patterns = []

# When true, bypass .gitignore and index all files. Equivalent to
# passing --include-ignored on the CLI. Not recommended for most projects.
include_ignored = false
```

---

## CLI Surface

### Updated command

```
clido index build [--include-ignored]
```

`--include-ignored`: bypass all ignore rules (`.gitignore`, `.git/info/exclude`, global gitignore, `.clido-ignore`) and index all files. Stats line notes that ignore rules were bypassed.

### Unchanged commands

`clido index search`, `clido index status`, and `clido index clear` are unaffected by this feature.

---

## `.clido-ignore` File Format

Same syntax as `.gitignore` (glob patterns, one per line, `#` comments, `!` negation). Location: project root (same directory as `.git/`). Processed by `ignore::WalkBuilder` via `add_custom_ignore_filename(".clido-ignore")`.

Example `.clido-ignore`:

```
# Large test fixtures we want in git but not in the clido index
tests/fixtures/large/
*.pb.go
generated/
```

`.clido-ignore` is not required. If absent, only `.gitignore` rules apply.

---

## Implementation Steps: `clido-index/src/lib.rs` File-Level Detail

The following functions in `lib.rs` are affected:

| Function | Change |
|---|---|
| `RepoIndex::build()` | Replace traversal with `ignore::WalkBuilder`. Add stats counters. |
| `RepoIndex::build_with_options()` | New function accepting `BuildOptions { include_ignored: bool }`. |
| `RepoIndex::new()` | Accept `IndexConfig` from `AgentConfig`. Pre-compile `exclude_patterns`. |

No changes to the SQLite schema. No changes to `RepoIndex::search()` or query functions.

---

## Test Plan

All tests in `crates/clido-index/tests/` using temp directories.

**`test_node_modules_excluded_by_default`**
Create a temp directory with `git init`, create `node_modules/lodash/index.js` and `src/main.rs`. Add `node_modules/` to `.gitignore`. Build index. Assert `src/main.rs` is indexed and `node_modules/lodash/index.js` is not.

**`test_target_dir_excluded_by_default`**
Create a temp git repo with a standard Rust `.gitignore` containing `/target`. Create `target/debug/clido` and `src/lib.rs`. Build index. Assert `src/lib.rs` is indexed and `target/debug/clido` is not.

**`test_git_dir_excluded`**
Create a temp git repo. Build index. Assert no paths containing `/.git/` appear in the index.

**`test_clido_ignore_file_respected`**
Create a temp git repo with `tests/fixtures/huge.bin` tracked by git (not in `.gitignore`). Create `.clido-ignore` containing `tests/fixtures/`. Build index. Assert `tests/fixtures/huge.bin` is not indexed.

**`test_clido_ignore_negation`**
Create `.clido-ignore` with:
```
tests/
!tests/unit/
```
Assert `tests/unit/` files are indexed but `tests/integration/` files are not.

**`test_include_ignored_bypasses_gitignore`**
Set up a repo where `node_modules/` is in `.gitignore`. Build index with `include_ignored = true`. Assert `node_modules/` files are indexed.

**`test_exclude_patterns_from_config`**
Set `exclude_patterns = ["*.min.js"]` in config. Create `dist/app.min.js` (not in `.gitignore`). Build index. Assert `dist/app.min.js` is not indexed.

**`test_nested_gitignore_monorepo`**
Create a monorepo structure:
```
packages/api/.gitignore   (contains: node_modules/)
packages/api/node_modules/express/index.js
packages/api/src/index.ts
packages/web/.gitignore   (contains: .next/)
packages/web/.next/cache/data.bin
packages/web/src/App.tsx
```
Build index from repo root. Assert `src/index.ts` and `src/App.tsx` are indexed; `node_modules/express/index.js` and `.next/cache/data.bin` are not.

**`test_stats_output_includes_skipped_count`**
Build index on a repo with `node_modules/` containing 10 files. Assert stats output contains `Skipped 10` (or more).

**`test_index_size_reduction`**
Measure SQLite file size before (with an old index built without ignore rules) and after re-building with ignore rules enabled on a fixture with `node_modules/`. Assert new index is at least 90% smaller.

---

## Docs Pages

### Update: `docs/guide/index-search.md`

Add a section "What gets indexed" explaining:
- Files excluded by `.gitignore` are skipped automatically.
- `.clido-ignore` at project root adds project-specific exclusions.
- `[index] exclude_patterns` in config adds user-wide exclusions.
- `clido index build --include-ignored` bypasses all rules.

Add a section "`.clido-ignore` file" with syntax reference and examples.

### Update: `docs/reference/config.md`

Add `[index]` section with `exclude_patterns` (type: array of strings, default: `[]`) and `include_ignored` (type: bool, default: `false`).

---

## Definition of Done

- [ ] `ignore::WalkBuilder` is used in `RepoIndex::build()` with `git_ignore(true)`.
- [ ] `.git/` directory contents are never indexed.
- [ ] `node_modules/` and `target/` are excluded when present in `.gitignore`.
- [ ] `.clido-ignore` file at project root is respected when present; absence is silently ignored.
- [ ] `[index] exclude_patterns` config key applies additional glob exclusions on top of `.gitignore`.
- [ ] `clido index build --include-ignored` bypasses all ignore rules and indexes all files.
- [ ] Stats line printed after every build: number indexed and number skipped.
- [ ] Nested `.gitignore` files in monorepo structures are respected.
- [ ] All eight test cases in the test plan pass.
- [ ] `docs/guide/index-search.md` documents `.clido-ignore` and `exclude_patterns`.
- [ ] CI passes (cargo test --workspace, cargo clippy --workspace).
