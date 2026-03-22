# Feature Plan: Edit Tool Reliability — Multi-Strategy Patching

**Status:** Planned
**Crate:** `clido-tools`
**Primary file:** `crates/clido-tools/src/edit.rs`
**Estimated effort:** Medium (3–5 days)

---

## Problem Statement

The current Edit tool performs a naive exact string match: it searches the file for `old_string` and replaces it with `new_string`. This design causes silent or incorrect failures in several real-world scenarios:

1. **Duplicate occurrences.** If `old_string` appears twice in the file, the tool replaces only the first match without warning. The model never knows it matched the wrong one.
2. **Whitespace drift.** The model frequently adds trailing spaces or changes indentation when generating `old_string`, especially when the file was shown via a tool call that included line numbers. A single extra space causes a full match failure with an unhelpful "not found" error.
3. **TOCTOU (Time-of-Check/Time-of-Use) race.** The model reads a file, generates an edit based on what it saw, then submits the Edit call. If another tool (e.g. Bash running `cargo fmt`) has already modified the file between Read and Edit, the edit is applied to the wrong content, silently corrupting the file.
4. **Mixed line endings.** Files with `\r\n` endings on Windows or in cloned repos cause exact matching to fail even when the text is visually identical.
5. **Unicode and BOM edge cases.** Files with a UTF-8 BOM or certain Unicode normalization forms cause byte-level mismatch even when visually identical.

These failures are the single largest category of silent agent errors. The agent spends tokens on retries, sometimes applies the wrong patch, and sometimes gives up with a misleading error message. Claude Code suffers the same problem but mitigates it with verbose prompting. Aider avoids it by using unified diffs, which encode context lines and line numbers rather than relying on exact string identity.

---

## Competitive Analysis

| Tool | Approach | Weakness |
|---|---|---|
| Claude Code (Edit) | Exact string match + detailed prompt instructions | Same TOCTOU and whitespace issues; relies on model self-discipline |
| Aider | Unified diff format | Higher token cost; complex patch format that models sometimes generate incorrectly |
| Cursor | IDE-native AST patching | Requires IDE integration; not applicable to a CLI agent |
| Cline | Whole-file replacement preferred | Eliminates match problem but wastes tokens on large files |

Our approach threads the needle: fast exact match when unambiguous, with automatic fallback to normalized and fuzzy matching, plus structural safeguards (TOCTOU hash guard, line-range hints) that competitors lack.

---

## Our Specific Improvements

### Three-Tier Matching Strategy

Every Edit call attempts tiers in sequence and stops at the first successful unambiguous match:

**Tier 1 — Exact match**
Byte-for-byte search. Zero false positives. If the match count is exactly 1, apply immediately. If match count is 0, fall through to Tier 2. If match count is 2+, return an ambiguous-match error immediately (do not fall through; ambiguity is always an error regardless of tier).

**Tier 2 — Normalized match**
Apply the following normalizations to both the file content and `old_string` before searching:
- Strip trailing whitespace from each line
- Normalize `\r\n` to `\n`
- Collapse leading whitespace to relative indentation (compute the minimum indent of all non-empty lines in `old_string`; subtract that many spaces from each line in both the query and candidate windows)

If a unique match is found after normalization, apply the replacement using the original file bytes (not the normalized version) so that the surrounding indentation is preserved.

**Tier 3 — Fuzzy match**
Use the `similar` crate's `TextDiff` algorithm (already a transitive dependency via `insta`; add it explicitly to `clido-tools/Cargo.toml`). Slide a window of ± `old_string.line_count() + 5` lines across the file and compute the similarity ratio for each window against `old_string`. The window with the highest similarity ratio above a threshold (0.82 by default) is the candidate. If exactly one window exceeds the threshold, apply the edit. If multiple windows are within 0.02 of each other, treat as ambiguous.

Each tier records which strategy it used. The tool output text includes `match_strategy: exact|normalized|fuzzy` and a `match_confidence` float (1.0 for exact, 0.90–0.99 for normalized, 0.82–0.99 for fuzzy). The model can read this in its context window and decide whether to verify.

### Line-Range Hints

New optional parameters `start_line` and `end_line` (1-indexed, inclusive) constrain the search window. When provided:
- Tier 1 only searches within that range
- Tier 2 only searches within that range
- Tier 3 uses the range center as the search anchor

This resolves the most common ambiguity: a boilerplate function that appears multiple times (e.g. `fn new()` or `fn default()`). The model reads the file, notes the line number, passes it as a hint.

### TOCTOU Hash Guard

When a file is opened via the Read tool, a `FileTracker` records `(absolute_path, modification_time, blake3_hash_of_content)`. At Edit write time, the guard:
1. Re-reads the file's current `mtime` and size.
2. If `mtime` differs from the recorded value, re-computes the blake3 hash.
3. If the hash differs, returns `ClidoError::FileChangedSinceRead { path, recorded_hash, current_hash }` before applying any patch.

This guard is opt-in via config (`[tools.edit] toctou_guard = true`, default true) so users who deliberately run concurrent tools can disable it.

### Replace-All Mode

The existing `replace_all: bool` parameter is unchanged. In replace-all mode, TOCTOU guard still runs, but ambiguity errors are suppressed (all matches are intentional).

---

## New Schema

```json
{
  "name": "Edit",
  "description": "Replace old_string with new_string in a file. Attempts exact, normalized, and fuzzy matching in sequence.",
  "parameters": {
    "file_path":   { "type": "string", "required": true },
    "old_string":  { "type": "string", "required": true },
    "new_string":  { "type": "string", "required": true },
    "replace_all": { "type": "boolean", "default": false },
    "start_line":  { "type": "integer", "description": "Hint: restrict search to lines >= this value (1-indexed)" },
    "end_line":    { "type": "integer", "description": "Hint: restrict search to lines <= this value (1-indexed)" },
    "strategy":    { "type": "string", "enum": ["exact", "normalized", "fuzzy"], "description": "Force a specific strategy. Default: auto (all tiers tried in sequence)" }
  }
}
```

---

## Tool Output Format

On success:
```
Edited /path/to/file.rs (lines 42–47)
match_strategy: normalized
match_confidence: 0.97
```

On ambiguous match error:
```
Error: old_string matches 3 locations. Provide start_line/end_line to disambiguate:
  - Line 42: fn new() { ... }
  - Line 88: fn new() { ... }
  - Line 134: fn new() { ... }
```

On no-match error:
```
Error: old_string not found in /path/to/file.rs (tried exact, normalized, fuzzy).
Closest partial match (similarity 0.61) near line 55:
--- expected
+++ found
 fn process(
-    input: &str,
+    input: String,
 ) -> Result<()> {
```

On TOCTOU error:
```
Error: File /path/to/file.rs was modified after it was last read.
Recorded hash: abc123...  Current hash: def456...
Re-read the file and regenerate the edit.
```

---

## Implementation Steps

### 1. Add `similar` dependency

In `crates/clido-tools/Cargo.toml`:
```toml
similar = "2.6"
blake3 = "1.5"
```

### 2. Create `FileTracker`

**File:** `crates/clido-tools/src/file_tracker.rs`

```rust
pub struct FileEntry {
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub hash: [u8; 32],  // blake3
}

pub struct FileTracker {
    entries: HashMap<PathBuf, FileEntry>,
}

impl FileTracker {
    pub fn record_read(&mut self, path: &Path, content: &[u8]);
    pub fn check_unchanged(&self, path: &Path) -> Result<(), ClidoError>;
}
```

The `FileTracker` is stored in `AgentSession` (in `clido-storage`) and passed into the tool execution context. The Read tool calls `tracker.record_read()` after loading a file. The Edit tool calls `tracker.check_unchanged()` before writing.

The execution context struct in `clido-agent/src/agent_loop.rs` gains a `file_tracker: Arc<Mutex<FileTracker>>` field passed to each tool's `execute()` call via the existing `ToolContext`.

### 3. Refactor `crates/clido-tools/src/edit.rs`

Extract matching logic into a `Matcher` struct with three methods:

```rust
struct Matcher<'a> {
    haystack: &'a str,
    needle: &'a str,
    start_line: Option<usize>,
    end_line: Option<usize>,
}

impl<'a> Matcher<'a> {
    fn match_exact(&self) -> MatchResult;
    fn match_normalized(&self) -> MatchResult;
    fn match_fuzzy(&self) -> MatchResult;
    fn run(&self, forced_strategy: Option<Strategy>) -> MatchResult;
}

enum MatchResult {
    Found { byte_start: usize, byte_end: usize, strategy: Strategy, confidence: f32 },
    Ambiguous { candidates: Vec<CandidateLocation> },
    NotFound { closest: Option<ClosestMatch> },
}
```

The `EditTool::execute()` method:
1. Reads file content.
2. Checks TOCTOU guard.
3. Instantiates `Matcher` with optional line-range hints.
4. Calls `matcher.run(forced_strategy)`.
5. On `Found`: reconstructs the new file content and writes it.
6. On `Ambiguous` or `NotFound`: returns an error with the diagnostic payload.

### 4. Update `EditTool::schema()`

Return the expanded JSON schema with the new optional fields.

### 5. Update `EditTool::description()`

Update the description string to mention multi-strategy matching and the new parameters. This description is included verbatim in the agent system prompt; the wording matters.

### 6. Wire `FileTracker` into `ToolContext`

**File:** `crates/clido-agent/src/tool_context.rs` (create if not exists, or add to `agent_loop.rs`)

Add `file_tracker: Arc<Mutex<FileTracker>>` to the context struct passed to all tools. Update the `Read` tool to call `record_read` after a successful file read.

### 7. Config additions

**File:** `crates/clido-core/src/config_loader.rs`

```toml
[tools.edit]
toctou_guard = true
fuzzy_threshold = 0.82
strategy = "auto"        # "auto" | "exact" | "normalized" | "fuzzy"
```

Add `EditConfig` struct to `AgentConfig` and deserialize from TOML.

---

## Error Variants

In `crates/clido-core/src/error.rs`, add:

```rust
pub enum ClidoError {
    // ... existing variants ...
    EditAmbiguousMatch { candidates: Vec<(usize, String)> },
    EditNoMatch { closest_similarity: f32, closest_preview: String },
    FileChangedSinceRead { path: PathBuf, recorded_hash: String, current_hash: String },
}
```

---

## CLI Surface

No new CLI flags. The new schema parameters are passed by the model via tool call JSON. Users do not invoke the Edit tool directly from the command line.

---

## TUI Changes

No new slash commands needed. The error messages from the new failure modes are displayed in the existing ChatLine output rendering. Consider adding a subtle indicator in the status bar when fuzzy matching was used (e.g., `~ fuzzy edit` in dim text) so the user can notice when the agent is struggling with a file.

---

## Config Schema

Full addition to `~/.clido/config.toml`:

```toml
[tools.edit]
# Enable TOCTOU file-change detection. Requires Read tool to be called before Edit.
toctou_guard = true

# Minimum similarity ratio for fuzzy match acceptance (0.0–1.0).
fuzzy_threshold = 0.82

# Default strategy: "auto" tries exact → normalized → fuzzy in sequence.
# "exact" disables fallback. "normalized" skips exact. "fuzzy" always uses fuzzy.
strategy = "auto"
```

---

## Test Plan

All tests in `crates/clido-tools/tests/edit_tests.rs` and `crates/clido-tools/src/edit.rs` (unit tests in `#[cfg(test)]` module).

```rust
// Unit tests for Tier 1
fn test_exact_match_single_occurrence()
fn test_exact_match_returns_correct_byte_range()
fn test_exact_match_unicode_multibyte()

// Unit tests for Tier 2
fn test_normalized_match_trailing_whitespace()
fn test_normalized_match_crlf_to_lf()
fn test_normalized_match_indentation_drift()
fn test_normalized_match_preserves_original_indentation_in_output()

// Unit tests for Tier 3
fn test_fuzzy_match_minor_edit()
fn test_fuzzy_match_rejects_low_similarity()
fn test_fuzzy_match_10k_line_file_performance()  // assert < 200ms

// Ambiguity tests
fn test_ambiguous_match_returns_all_candidate_lines()
fn test_line_range_hint_resolves_ambiguity()
fn test_line_range_hint_start_only()
fn test_line_range_hint_wrong_range_falls_back()

// TOCTOU tests
fn test_toctou_detects_file_changed_after_read()
fn test_toctou_passes_when_file_unchanged()
fn test_toctou_disabled_via_config()

// No-match diagnostic tests
fn test_no_match_shows_closest_partial()
fn test_no_match_on_empty_file()

// Replace-all tests
fn test_replace_all_replaces_every_occurrence()
fn test_replace_all_skips_ambiguity_check()

// Strategy override tests
fn test_forced_strategy_exact_skips_normalized()
fn test_forced_strategy_fuzzy_skips_exact_and_normalized()

// Mixed line ending tests
fn test_crlf_file_exact_match_with_lf_query()
fn test_mixed_line_endings_within_single_file()
```

---

## Docs Pages to Update / Create

- **Update** `docs/developer/adding-tools.md` — add section "Multi-strategy matching and FileTracker" explaining how new tools should call `tracker.record_read()` and `tracker.check_unchanged()`.
- **Update** `docs/reference/tools/edit.md` — document new parameters (`start_line`, `end_line`, `strategy`), explain three-tier matching, show error message examples.
- **Update** `docs/guide/editing-files.md` — user-facing explanation of why fuzzy matching sometimes reports confidence < 1.0 and how to provide line hints.
- **Update** agent system prompt in `crates/clido-agent/src/agent_loop.rs` or wherever it is assembled — add a paragraph explaining the three-tier strategy and instructing the model to always pass `start_line` when the string being replaced is non-unique.

---

## Definition of Done

- [ ] `Matcher` struct implements all three tiers with unit tests passing for each
- [ ] `EditTool::schema()` returns the updated JSON schema including `start_line`, `end_line`, `strategy`
- [ ] `EditTool::description()` updated to mention multi-strategy matching
- [ ] `FileTracker` struct created in `clido-tools`; `record_read` called by `ReadTool`; `check_unchanged` called by `EditTool`
- [ ] `ClidoError::EditAmbiguousMatch`, `EditNoMatch`, `FileChangedSinceRead` variants added and matched in all error formatting paths
- [ ] TOCTOU guard defaults to `true`; config flag `[tools.edit] toctou_guard` toggles it
- [ ] `fuzzy_threshold` and `strategy` configurable via `[tools.edit]` TOML section
- [ ] All 30+ test functions listed above pass
- [ ] 10k-line file fuzzy match completes in < 200ms (verified with `#[test]` timing assertion)
- [ ] Error messages for ambiguous and no-match cases include actionable diagnostic info (candidate line numbers, partial diff)
- [ ] `docs/reference/tools/edit.md` updated with new parameters
- [ ] `docs/developer/adding-tools.md` updated with FileTracker usage guidance
- [ ] Agent system prompt updated to instruct model on `start_line` usage
- [ ] No regressions in existing `replace_all` behavior
- [ ] `cargo clippy -- -D warnings` passes on `clido-tools`
