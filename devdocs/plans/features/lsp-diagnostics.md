# Feature Plan: LSP Diagnostics and Compiler Error Access

**Status:** Planned
**Crate:** `clido-tools`
**Primary file:** `crates/clido-tools/src/diagnostics.rs`
**Estimated effort:** Medium-Large (4–6 days)

---

## Problem Statement

Every professional coding workflow involves a tight feedback loop between writing code and seeing compiler or linter errors. The current Clido agent handles this the same way a developer would at a terminal: it runs `cargo check` or `tsc` via the Bash tool, then reads the raw text output and parses it mentally.

This approach has three concrete problems:

1. **Speed.** `cargo check` on a cold workspace can take 5–30 seconds. The agent triggers this via Bash, waits for the full output, and the entire output lands in the context window as unstructured text. This burns tokens and slows the loop.
2. **Fragility.** Compiler output formats differ between versions and across languages. Rust's human-readable output is completely different from its `--message-format json` output. TypeScript's `tsc` changes its output format between versions. The model has to parse this inconsistently-formatted text rather than reasoning about structured data.
3. **No incremental filtering.** When the agent asks "are there any errors in this file?" it gets all output for all files. There is no way to query "just errors in `src/auth.rs`" without post-processing.

Cline and Cursor integrate with the IDE's language server (LSP) to receive diagnostics as structured JSON events. This is powerful but requires an embedded IDE or a running LSP daemon. Clido is a CLI tool; we cannot assume a running language server. Our solution gets 80% of the benefit without the complexity: call the compiler in its JSON output mode and normalize the result.

---

## Competitive Analysis

| Tool | Approach | Weakness |
|---|---|---|
| Cursor | Full LSP integration via IDE | Requires IDE; not portable to CLI |
| Cline (VS Code) | Reads VS Code diagnostics API | Requires VS Code; tied to IDE lifecycle |
| Claude Code | Bash + raw text | Same as current Clido; model parses text |
| Aider | Bash + regex parsing per-language | Brittle; breaks on version changes |
| Clido (current) | Bash + raw text | Slow, fragile, unstructured |

Our approach: a language-agnostic `DiagnosticsTool` that invokes compilers/linters in their native structured-output modes and normalizes results to a single JSON schema. No LSP daemon, no IDE dependency, no regex parsing of human-readable output.

---

## Our Specific Improvements

### Structured Output from Native Tooling

Every major language toolchain supports a machine-readable output mode. We use these:

| Language | Command | Output format |
|---|---|---|
| Rust | `cargo check --message-format json` | NDJSON, one JSON object per line |
| TypeScript | `tsc --noEmit --pretty false` | Plain text with known structure; supplemented by `typescript-json-diagnostics` wrapper if available |
| JavaScript/TypeScript | `eslint --format json <path>` | JSON array |
| Python | `pyright --outputjson` | JSON object with `generalDiagnostics` array |
| Go | `go vet ./... 2>&1` + `go build ./... 2>&1` | Text with `file:line:col: message` pattern (no native JSON; use structured regex on this specific stable format) |
| Custom | `[tools.diagnostics] command = "..."` | Raw; attempt JSON parse; fallback to line-by-line |

Each backend has its own parser in `crates/clido-tools/src/diagnostics/backends/`.

### Normalized Diagnostic Schema

Every backend produces output normalized to this structure:

```rust
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: u32,          // 1-indexed
    pub col: u32,           // 1-indexed
    pub severity: Severity, // Error | Warning | Info | Hint
    pub code: Option<String>, // "E0502", "TS2345", "F821", etc.
    pub message: String,
    pub source: String,     // "rustc", "tsc", "eslint", "pyright", "go-vet"
}

pub enum Severity { Error, Warning, Info, Hint }
```

The tool returns a JSON array of `Diagnostic` objects plus a summary string. The model receives clean structured data it can reason about directly, with no text parsing needed.

### Auto-Detection Logic

The tool scans the workspace root (defaulting to the current working directory) for marker files in priority order:

1. `Cargo.toml` → Rust
2. `tsconfig.json` → TypeScript
3. `package.json` (with `eslint` in devDependencies) → ESLint
4. `pyproject.toml` or `setup.py` → Python/Pyright
5. `go.mod` → Go

Multiple markers can be present; each detected language gets its own backend run. Results are merged and returned together. The `language` input parameter overrides auto-detection.

### Caching

Results are cached in memory (in `DiagnosticsCache`, stored in `AgentSession`) keyed by `(workspace_root, language, path_filter)`. Cache entries expire after 30 seconds. The `EditTool` and `WriteTool` call `cache.invalidate_for_file(path)` after a successful write so the next `Diagnostics` call always re-runs. This is the main integration point between `DiagnosticsTool` and the rest of the tool suite.

Cache invalidation uses the same `FileTracker` introduced by the Edit Reliability feature. If `FileTracker` is not available (feature not yet deployed), invalidation falls back to time-based expiry only.

---

## `DiagnosticsTool` Design

**File:** `crates/clido-tools/src/diagnostics.rs`

```rust
pub struct DiagnosticsTool {
    config: DiagnosticsConfig,
    cache: Arc<Mutex<DiagnosticsCache>>,
}

impl Tool for DiagnosticsTool {
    fn name(&self) -> &str { "Diagnostics" }
    fn description(&self) -> &str { "Run compiler/linter checks and return structured diagnostics." }
    fn is_read_only(&self) -> bool { true }
    fn schema(&self) -> serde_json::Value { /* see below */ }
    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> Result<String, ClidoError>;
}
```

### Input Schema

```json
{
  "name": "Diagnostics",
  "parameters": {
    "language": {
      "type": "string",
      "enum": ["rust", "typescript", "javascript", "python", "go", "auto"],
      "default": "auto",
      "description": "Language to check. 'auto' detects from workspace files."
    },
    "path": {
      "type": "string",
      "description": "Optional: restrict diagnostics to this file or directory path."
    },
    "severity_filter": {
      "type": "string",
      "enum": ["error", "warning", "all"],
      "default": "all",
      "description": "Return only diagnostics at or above this severity."
    }
  }
}
```

### Output Format

```
3 errors, 2 warnings in 2 files

[
  {
    "file": "src/auth.rs",
    "line": 42,
    "col": 15,
    "severity": "error",
    "code": "E0502",
    "message": "cannot borrow `state` as mutable because it is also borrowed as immutable",
    "source": "rustc"
  },
  ...
]
```

If no diagnostics: `"0 errors, 0 warnings — all checks passed."`

---

## Backend Implementation

### Directory Structure

```
crates/clido-tools/src/diagnostics/
    mod.rs          — DiagnosticsTool, DiagnosticsConfig, DiagnosticsCache
    backends/
        mod.rs      — BackendTrait, BackendResult
        rust.rs     — RustBackend: runs `cargo check --message-format json`
        typescript.rs — TypeScriptBackend: runs `tsc --noEmit --pretty false`
        eslint.rs   — EsLintBackend: runs `eslint --format json`
        pyright.rs  — PyrightBackend: runs `pyright --outputjson`
        go.rs       — GoBackend: runs `go vet` + `go build`, parses structured text
        custom.rs   — CustomBackend: runs user-configured command
    cache.rs        — DiagnosticsCache: time-based + file-invalidation cache
    normalize.rs    — Diagnostic struct, Severity enum, normalization helpers
    detect.rs       — workspace language auto-detection logic
```

### `BackendTrait`

```rust
#[async_trait]
pub trait BackendTrait: Send + Sync {
    fn language(&self) -> &str;
    fn is_available(&self) -> bool;  // check if binary exists in PATH
    async fn run(
        &self,
        workspace: &Path,
        path_filter: Option<&Path>,
        config: &DiagnosticsConfig,
    ) -> Result<Vec<Diagnostic>, ClidoError>;
}
```

### Rust Backend Detail

`cargo check --message-format json` outputs one JSON object per line. Each line is a `cargo_metadata::Message`. We care about `Message::CompilerMessage` which contains a `rustc_serialize::Diagnostic` with `spans` (file, line_start, column_start), `level` (error/warning/note/help), `code.code`, and `message`. Filter out `note` and `help` severity levels unless `severity_filter = "all"`.

### TypeScript Backend Detail

`tsc --noEmit --pretty false` outputs lines in the format:
`path/to/file.ts(line,col): error TS2345: message text`

This format has been stable since TypeScript 2.x. Parse it with a single regex. Fall back to this even if a JSON wrapper tool is available, to avoid requiring an extra npm install.

### Go Backend Detail

`go vet ./...` outputs `path/file.go:line:col: message` on stderr. `go build ./...` outputs the same format. Both are stable. Parse with:
```
^(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+):\s+(?P<message>.+)$
```

---

## Timeout Handling

Each backend is run inside `tokio::time::timeout(Duration::from_secs(config.timeout_secs), backend.run(...))`. On timeout:
- Return a `ClidoError::DiagnosticsTimeout { language, timeout_secs }` error.
- The tool output contains: `"Diagnostics timed out after 60s for rust. Consider running in a smaller path scope."`

Default timeout: 60 seconds. Per-language override: `[tools.diagnostics.timeouts] rust = 120`.

---

## Integration with Other Tools

### `WriteTool` and `EditTool` invalidation

After a successful write, both tools call:
```rust
ctx.diagnostics_cache.lock().await.invalidate_for_file(&absolute_path);
```

This requires `diagnostics_cache: Arc<Mutex<DiagnosticsCache>>` in `ToolContext`.

### `FileTracker` integration

`DiagnosticsTool` records the workspace state hash when it caches results, enabling smarter invalidation: if the set of files in the workspace hasn't changed since the last cache entry, the cache is valid regardless of the 30s TTL.

---

## TUI Changes

### New `/check` Slash Command

**File:** `crates/clido-cli/src/tui.rs`

Add `"/check"` to the `SLASH_COMMANDS` array and handle it in the slash-command dispatch block:

```rust
"/check" => {
    // Run DiagnosticsTool for the current workspace
    // Display results in a dedicated ChatLine::DiagnosticsResult variant
}
```

`ChatLine::DiagnosticsResult { summary: String, diagnostics: Vec<Diagnostic> }` renders as:
```
[Diagnostics] 3 errors, 2 warnings in 2 files
  src/auth.rs:42:15  error E0502  cannot borrow `state` as mutable...
  src/auth.rs:88:5   warning      unused variable `token`
  src/lib.rs:12:1    error E0308  mismatched types
```

Errors render in red, warnings in yellow, using ratatui's `Style::default().fg(Color::Red)`.

### Status Bar Indicator

When diagnostics have been run and there are active errors, show `[E:3 W:2]` in the status bar. This updates after each `/check` and after any `DiagnosticsTool` call made by the agent. Clear the indicator when a new `/check` returns zero errors.

---

## CLI Surface

No special CLI flag needed. `DiagnosticsTool` is registered in the tool registry alongside `ReadTool`, `WriteTool`, etc. The agent discovers it automatically and can call it during any session.

For scripted use, users can run: `clido run "check for type errors in src/"` and the agent will invoke `DiagnosticsTool` as appropriate.

---

## Config Schema

Full addition to `~/.clido/config.toml`:

```toml
[tools.diagnostics]
# Enable the Diagnostics tool
enabled = true

# Backends to use. "auto" detects from workspace. Explicit list runs only those.
backends = ["auto"]

# Default timeout for each backend check in seconds
timeout_secs = 60

# Per-language timeout overrides
[tools.diagnostics.timeouts]
rust = 90
typescript = 30
go = 30

# Cache TTL in seconds (0 to disable caching)
cache_ttl_secs = 30
```

Add `DiagnosticsConfig` struct to `AgentConfig` in `crates/clido-core/src/config_loader.rs`.

---

## Error Variants

In `crates/clido-core/src/error.rs`, add:

```rust
pub enum ClidoError {
    // ... existing ...
    DiagnosticsTimeout { language: String, timeout_secs: u64 },
    DiagnosticsBackendNotFound { language: String, binary: String },
    DiagnosticsParseError { language: String, raw_output: String, parse_error: String },
}
```

---

## Test Plan

All tests in `crates/clido-tools/tests/diagnostics_tests.rs` and unit tests in the backends.

```rust
// Auto-detection tests
fn test_detect_rust_from_cargo_toml()
fn test_detect_typescript_from_tsconfig()
fn test_detect_python_from_pyproject_toml()
fn test_detect_go_from_go_mod()
fn test_detect_multiple_languages_in_same_workspace()
fn test_detect_falls_back_to_custom_if_configured()

// Rust backend parser tests
fn test_rust_backend_parses_error_message()
fn test_rust_backend_parses_warning_message()
fn test_rust_backend_filters_note_and_help()
fn test_rust_backend_handles_empty_output()
fn test_rust_backend_handles_workspace_with_no_errors()

// TypeScript backend parser tests
fn test_typescript_backend_parses_error_line()
fn test_typescript_backend_parses_multiline_message()
fn test_typescript_backend_handles_no_config_file_error()

// ESLint backend parser tests
fn test_eslint_backend_parses_json_array()
fn test_eslint_backend_handles_empty_json_array()

// Pyright backend parser tests
fn test_pyright_backend_parses_outputjson()
fn test_pyright_backend_severity_mapping()

// Go backend parser tests
fn test_go_backend_parses_vet_output()
fn test_go_backend_parses_build_output()

// Severity filter tests
fn test_severity_filter_error_only()
fn test_severity_filter_warning_includes_errors()
fn test_severity_filter_all_includes_hints()

// Cache tests
fn test_cache_returns_cached_result_within_ttl()
fn test_cache_expires_after_ttl()
fn test_cache_invalidates_on_edit_tool_write()
fn test_cache_invalidates_on_write_tool_write()

// Timeout tests
fn test_timeout_returns_error_after_configured_seconds()
fn test_timeout_per_language_override()

// Path filter tests
fn test_path_filter_restricts_to_single_file()
fn test_path_filter_restricts_to_directory()

// Integration: backend not available
fn test_backend_not_found_returns_clear_error()
```

---

## Docs Pages to Update / Create

- **Create** `docs/guide/diagnostics.md` — user-facing guide explaining the Diagnostics tool, the `/check` slash command, supported languages, and how to configure backends and timeouts.
- **Create** `docs/reference/tools/diagnostics.md` — reference page with full schema, output format examples, and error messages.
- **Update** `docs/developer/adding-tools.md` — add section on integrating with `DiagnosticsCache` for cache invalidation.
- **Update** `docs/reference/configuration.md` — document the `[tools.diagnostics]` config section.
- **Update** `docs/guide/slash-commands.md` — add `/check` with description and example output.

---

## Definition of Done

- [ ] `DiagnosticsTool` struct implements `Tool` trait with correct `name()`, `is_read_only() = true`, `schema()`, and `execute()`
- [ ] All five language backends implemented: `rust.rs`, `typescript.rs`, `eslint.rs`, `pyright.rs`, `go.rs`
- [ ] `CustomBackend` supports `[tools.diagnostics] command` override
- [ ] Auto-detection logic correctly identifies language from workspace marker files
- [ ] All diagnostic output normalized to `Diagnostic` struct with `file`, `line`, `col`, `severity`, `code`, `message`, `source` fields
- [ ] `severity_filter` input parameter correctly filters results
- [ ] `path` input parameter correctly filters results to a file or directory
- [ ] Caching implemented with 30s TTL and file-write invalidation from `EditTool` and `WriteTool`
- [ ] Timeout enforced per backend; `DiagnosticsTimeout` error returned on expiry
- [ ] `DiagnosticsConfig` deserialized from `[tools.diagnostics]` TOML section in `AgentConfig`
- [ ] `/check` slash command added to TUI; results displayed with severity-colored output
- [ ] Status bar shows `[E:N W:N]` indicator after diagnostics run
- [ ] `DiagnosticsTimeout`, `DiagnosticsBackendNotFound`, `DiagnosticsParseError` error variants added
- [ ] All 32+ test functions listed above pass
- [ ] `docs/guide/diagnostics.md` created
- [ ] `docs/reference/tools/diagnostics.md` created
- [ ] `docs/reference/configuration.md` updated with `[tools.diagnostics]` section
- [ ] `cargo clippy -- -D warnings` passes on `clido-tools`
