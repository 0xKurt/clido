# Feature Plan: Diff Preview and Approval Before Write

**Status:** Planned
**Target release:** V2
**Crates affected:** `clido-tools`, `clido-agent`, `clido-cli`, `clido-core`
**Docs affected:** `docs/guide/running-prompts.md`, `docs/reference/flags.md`, `docs/reference/config.md`

---

## Problem Statement

When the agent calls `WriteTool` or `EditTool`, the file is written immediately. There is no pre-write preview, no diff, and no confirmation step in the default permission mode. The current `ChatLine::Diff` variant renders the diff *after* the fact — informational only, not actionable. A user reading the TUI sees the change has already happened.

This matters because:

1. **Accidental overwrites**: a confused or hallucinating agent can overwrite a file with subtly wrong content. By the time the user reads the post-hoc diff, it has already happened.
2. **No rollback in the TUI**: there is no undo button. The user must manually restore from git or from memory.
3. **Large edits are hard to review after the fact**: when the agent edits a 300-line function, the post-hoc diff scrolls past before the user can read it.
4. **Trust gap for new users**: a new user trying clido for the first time who sees files changing without confirmation is likely to distrust the tool and stop using it.

### Competitive Landscape

**Cline** shows a full diff preview for every `write_to_file` and `replace_in_file` call. The user sees the colored diff and must click Accept or Reject before anything is written. This is the gold standard for safe AI file editing. The downside: it interrupts flow for every single change, even trivial ones.

**Claude Code** does not show a pre-write diff by default. It writes immediately but shows the diff afterward in the terminal. In `--plan` mode it shows what it *would* do without executing. It relies on git for rollback.

**Aider** writes immediately but auto-stages every change, so `git diff HEAD` always gives you a rollback path. It doesn't block on user confirmation for individual file edits.

**Cursor** shows inline diffs in the editor; the user can accept or reject individual hunks in the IDE UI.

### Our Approach

Clido introduces a new **`diff-review` permission mode** that sits between `default` and `plan-only`. In `diff-review` mode, Write and Edit operations show a colored unified diff in the existing TUI permission modal before applying. The user can Accept, Reject, or open the proposed content in `$EDITOR`.

Importantly, this is not a new UI pattern. It reuses the existing permission modal component in `tui.rs`. The diff is embedded in the modal body, not a separate screen. This keeps the implementation minimal and the UX consistent.

`default` mode behavior is unchanged: the agent writes files freely for non-destructive operations. Users who want confirmation switch to `diff-review`.

---

## Design Decisions and Rationale

### Reuse the existing permission modal, don't add a new screen

The TUI already has a permission modal that blocks agent execution until the user responds. Adding a diff to this modal is less than 50 lines of new TUI code. Creating a separate "diff review screen" with its own navigation and state machine would double the UI complexity. Consistent with the principle: same modal component, new content type.

### Compute the diff in the tool, before calling `AskUser`

The tool knows the old content (it just read the file) and the new content (the agent provided it). Computing the diff in the tool (using the `similar` crate for unified diffs) means the permission system receives a pre-rendered diff string. The TUI does not need to understand file paths or content — it just renders the string with color coding. This keeps the permission modal's rendering logic generic.

### `diff: Option<String>` field on `PermRequest`

The existing `PermRequest` struct may have a `preview` field (check at implementation time). If so, consolidate: rename `preview` to `diff` or repurpose it to carry the unified diff string. If not, add `diff: Option<String>`. The TUI modal reads this field and renders it when present, regardless of permission mode. In `default` mode, `diff` is still populated but the modal is not shown for non-destructive writes — the diff appears in the post-hoc `ChatLine::Diff` instead.

### `EditInEditor` variant on `PermGrant`

```rust
pub enum PermGrant {
    Allow,
    Deny,
    EditInEditor,     // new
    AllowAll,         // already exists or add now
}
```

When the tool receives `EditInEditor`, it writes proposed content to a temp file (`tempfile::NamedTempFile`), spawns `$EDITOR` (falling back to `$VISUAL`, then `vi`), waits for the process to exit, reads the temp file, and uses that content as the final write. This gives the user surgical control over what the agent wrote.

### Streaming diffs (non-blocking render)

In `diff-review` mode, the diff is shown as soon as the tool computes it — before the agent's full response is complete. The agent loop is paused at the tool-call boundary (same as any permission check). This is already how permission checks work; no new streaming logic is needed.

### `accept-all` within a session

In `diff-review` mode, if the user presses `A` (Accept All) instead of `a` (Accept Once), all subsequent Write/Edit calls in this session bypass the modal. This is stored as a session-scoped flag, not persisted to config. It is the escape hatch for users who trust the agent after reviewing the first few changes.

---

## New Permission Mode: `diff-review`

### Mode Behavior Matrix

| Operation | `default` | `diff-review` | `plan-only` |
|---|---|---|---|
| `ReadTool` | Allow silently | Allow silently | Allow silently |
| `GlobTool` / `GrepTool` | Allow silently | Allow silently | Allow silently |
| `BashTool` (read-like) | Allow silently | Allow silently | Deny / show plan |
| `WriteTool` (new file) | Allow silently | Show diff modal | Deny / show plan |
| `WriteTool` (overwrite) | Ask for confirmation | Show diff modal | Deny / show plan |
| `EditTool` | Allow silently | Show diff modal | Deny / show plan |
| `BashTool` (destructive) | Ask for confirmation | Ask for confirmation | Deny / show plan |

In `diff-review` mode, the diff modal is shown for every Write and Edit, not just overwrite-of-existing-file. Creating a new file still shows the full proposed content as a diff (no `-` lines, all `+` lines).

### Setting the mode

Via CLI flag:

```
clido run --permission-mode diff-review "refactor the auth module"
```

Via config:

```toml
[agent]
permission_mode = "diff-review"
```

---

## Architecture

### Changes to `PermRequest`

In `crates/clido-agent/src/permissions.rs` (or wherever `PermRequest` lives):

```rust
pub struct PermRequest {
    pub tool_name: String,
    pub description: String,
    pub diff: Option<String>,          // unified diff string, pre-rendered
    pub proposed_content: Option<String>, // full proposed file content (for EditInEditor)
    pub file_path: Option<PathBuf>,    // for temp file creation in EditInEditor
}
```

### Changes to `AskUser` trait

In `crates/clido-agent/src/ask_user.rs`:

```rust
#[async_trait]
pub trait AskUser: Send + Sync {
    async fn ask(&self, req: PermRequest) -> PermGrant;
}
```

The signature is unchanged. `PermGrant::EditInEditor` is a new variant. The TUI implementation of `AskUser` renders the diff from `req.diff` in the modal body and maps key presses to `PermGrant` variants.

### Changes to `WriteTool`

In `crates/clido-tools/src/write.rs`:

1. Before writing, read the existing file content (if it exists). If the file does not exist, `old = ""`.
2. Compute unified diff: `similar::TextDiff::from_lines(old, new).unified_diff().to_string()`.
3. Build `PermRequest { tool_name: "write", diff: Some(diff_str), proposed_content: Some(new_content), file_path: Some(path), ... }`.
4. In `diff-review` mode: call `ask_user.ask(req).await`.
5. On `PermGrant::Allow`: write the file.
6. On `PermGrant::Deny`: return `ToolResult::Ok("Write rejected by user.")` (not an error; this is expected behavior).
7. On `PermGrant::EditInEditor`: write temp file, spawn editor, read result, write that result to the actual path.
8. On `PermGrant::AllowAll`: set session flag, write the file.

### Changes to `EditTool`

In `crates/clido-tools/src/edit.rs`:

Same pattern. The `old` and `new` strings are already computed as part of the edit operation (find-and-replace). The diff is the unified diff of the full file before and after the replacement.

### Changes to `TuiAskUser`

In `crates/clido-cli/src/tui.rs`, the `TuiAskUser` struct (which implements `AskUser`) is updated:

1. When `req.diff` is `Some`, render the diff in the modal body using ratatui styled spans: lines starting with `+` get `Style::new().fg(Color::Green)`, lines starting with `-` get `Style::new().fg(Color::Red)`, `@@` hunk headers get `Style::new().fg(Color::Cyan)`, unchanged lines are unstyled.
2. Modal footer shows choices based on context:
   - In `diff-review` mode with a diff: `[a] Accept  [A] Accept All  [r] Reject  [e] Edit in $EDITOR`
   - In `default` mode (confirm destructive): `[y] Yes  [n] No`
3. Key handler maps:
   - `a` / `y` / `Enter` → `PermGrant::Allow`
   - `A` → `PermGrant::AllowAll`
   - `r` / `n` / `Esc` → `PermGrant::Deny`
   - `e` → `PermGrant::EditInEditor`

The diff rendering area in the modal is scrollable if the diff is longer than the modal height. Use ratatui's `Scrollbar` widget (already available in the current ratatui version).

### `EditInEditor` implementation in tools

Both `WriteTool` and `EditTool` handle `PermGrant::EditInEditor` with shared code in a new function `crates/clido-tools/src/editor.rs`:

```rust
pub fn open_in_editor(proposed: &str, file_path: &Path) -> Result<String, ToolError> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let mut tmp = tempfile::Builder::new()
        .suffix(file_path.extension().and_then(|e| e.to_str()).unwrap_or(".txt"))
        .tempfile()?;
    tmp.write_all(proposed.as_bytes())?;
    let tmp_path = tmp.path().to_owned();
    tmp.keep()?; // prevent deletion while editor is open

    let status = std::process::Command::new(&editor)
        .arg(&tmp_path)
        .status()?;

    if !status.success() {
        return Err(ToolError::EditorFailed(editor));
    }

    let edited = std::fs::read_to_string(&tmp_path)?;
    std::fs::remove_file(&tmp_path).ok();
    Ok(edited)
}
```

This is synchronous because it must block for the editor to exit. Called from an async context via `tokio::task::spawn_blocking`.

### `similar` crate for diff computation

Add to `clido-tools/Cargo.toml`:

```toml
similar = { version = "2", features = ["text"] }
```

Use `similar::TextDiff::from_lines(old, new).unified_diff().header("a/path", "b/path").to_string()`.

### `tempfile` crate

Add to `clido-tools/Cargo.toml`:

```toml
tempfile = "3"
```

---

## Configuration Schema

In `config.toml`:

```toml
[agent]
# Permission mode controls how aggressively clido asks for confirmation.
# Values: "default" | "diff-review" | "plan-only" | "permissive"
# diff-review: show a diff preview modal before every Write and Edit operation.
permission_mode = "default"
```

The `permission_mode` key already exists for other values. `diff-review` is a new valid value.

---

## CLI Surface

### Updated `--permission-mode` flag

The existing `--permission-mode` flag (or `--perm` shorthand if already present) gains `diff-review` as a valid value.

```
clido run --permission-mode diff-review "..."
```

Updated help text:

```
--permission-mode <MODE>
    Control how the agent asks for permission before taking actions.
    Modes:
      default      Ask before destructive operations (default)
      diff-review  Show a diff preview modal before every Write or Edit
      plan-only    Show a plan, do not execute any writes
      permissive   Execute all operations without asking
```

### No new subcommands

This feature adds no new top-level subcommands. It extends the permission system.

---

## TUI Changes

| Location | Change |
|---|---|
| `tui.rs` permission modal | Add diff rendering with color-coded lines when `req.diff` is `Some` |
| `tui.rs` key handler | Add `a` → Allow, `A` → AllowAll, `e` → EditInEditor bindings |
| `tui.rs` modal footer | Show `[a] Accept [A] Accept All [r] Reject [e] Edit` when in diff-review mode |
| `tui.rs` modal body | Scrollable area for diff content using ratatui `Scrollbar` |
| `tui.rs` session state | Add `accept_all: bool` flag; when set, skip modal for remaining Write/Edit in session |

The modal height is dynamically sized to show up to 30 diff lines, then scrollable. Short diffs (< 10 lines) show in a compact modal; long diffs expand to fill most of the terminal height.

---

## Implementation Steps (ordered)

1. Add `similar` and `tempfile` to `clido-tools/Cargo.toml`.
2. Create `crates/clido-tools/src/editor.rs` with `open_in_editor()`.
3. Add `EditInEditor` and `AllowAll` variants to `PermGrant` enum.
4. Add `diff: Option<String>` and `proposed_content: Option<String>` and `file_path: Option<PathBuf>` to `PermRequest`.
5. Add `diff-review` variant to the `PermissionMode` enum in `clido-core`.
6. Update `WriteTool`: compute diff before write, build `PermRequest`, handle all `PermGrant` variants.
7. Update `EditTool`: compute diff before apply, build `PermRequest`, handle all `PermGrant` variants.
8. Update `TuiAskUser::ask()` in `tui.rs`: render diff, add key bindings, handle `EditInEditor`.
9. Add `accept_all` session flag to TUI state; implement `PermGrant::AllowAll` flow.
10. Update `PermissionMode` parsing in `config_loader.rs` to accept `"diff-review"`.
11. Update `--permission-mode` flag documentation in `cli.rs`.
12. Write all tests (see Test Plan below).
13. Update `docs/guide/running-prompts.md`.
14. Update `docs/reference/flags.md`.
15. Update `docs/reference/config.md`.

---

## Test Plan

### Unit tests: diff computation (`clido-tools`)

**`test_diff_computed_for_existing_file`**
Create a temp file with known content. Call `WriteTool` with new content in `diff-review` mode. Intercept `PermRequest`. Assert `req.diff` is `Some` and contains `+` and `-` lines reflecting the change.

**`test_diff_all_additions_for_new_file`**
Call `WriteTool` with a path that does not yet exist. Assert `req.diff` contains only `+` lines (no `-` lines).

**`test_diff_computed_for_edit_tool`**
Call `EditTool` with `old_string` and `new_string` in `diff-review` mode. Assert `req.diff` is `Some` and the hunk shows the replacement.

### Unit tests: `PermGrant` handling in tools

**`test_write_denied_does_not_write_file`**
Mock `AskUser` to return `PermGrant::Deny`. Call `WriteTool`. Assert the target file was not created or modified.

**`test_write_allowed_writes_file`**
Mock `AskUser` to return `PermGrant::Allow`. Call `WriteTool`. Assert the target file has the new content.

**`test_write_edit_in_editor_applies_edited_content`**
Mock `AskUser` to return `PermGrant::EditInEditor`. Mock `open_in_editor()` to return a modified version of proposed content. Assert the target file contains the editor's version, not the agent's version.

**`test_write_allow_all_skips_subsequent_modals`**
Mock `AskUser` to return `PermGrant::AllowAll` on the first call. Call `WriteTool` three times in the same session. Assert `AskUser::ask()` was called exactly once.

**`test_edit_denied_leaves_file_unchanged`**
Write a file with known content. Mock `AskUser` to return `PermGrant::Deny`. Call `EditTool`. Assert file content is unchanged.

### Integration tests: permission mode end-to-end

**`test_diff_review_mode_blocks_write_until_approved`**
Run agent with `permission_mode = "diff-review"`. Agent calls `WriteTool`. Assert the tool call is suspended until `AskUser::ask()` resolves.

**`test_default_mode_does_not_show_diff_modal_for_new_file`**
Run agent with `permission_mode = "default"`. Agent creates a new file. Assert `AskUser::ask()` is never called (no confirmation needed for new file in default mode).

**`test_accept_all_via_tui_key_A`**
In TUI integration test, send `A` keypress to the diff modal. Assert `accept_all` session flag is set. Assert the next Write call does not trigger a modal.

**`test_plan_only_mode_unaffected`**
Run agent with `permission_mode = "plan-only"`. Assert `WriteTool` still never writes files (plan-only behavior unchanged by this feature).

### Unit tests: `editor.rs`

**`test_open_in_editor_returns_edited_content`**
Set `EDITOR` to a shell script that appends a known string to the temp file. Call `open_in_editor("original", path)`. Assert returned string is `"originalAPPENDED"`.

**`test_open_in_editor_fallback_when_editor_not_set`**
Unset `EDITOR` and `VISUAL`. Mock the fallback (`vi`) to succeed. Assert no panic.

### TUI rendering tests

**`test_diff_lines_colored_in_modal`**
Render a permission modal with a diff string containing `+added` and `-removed` lines. Assert the rendered buffer contains green spans for `+` lines and red spans for `-` lines.

---

## Docs Pages

### Update: `docs/guide/running-prompts.md`

Add section: "Permission modes" (or update existing section if present).

Content:
- Table of all four modes (`default`, `diff-review`, `plan-only`, `permissive`) with descriptions.
- Explain `diff-review`: "Before any file write or edit, the TUI shows a colored diff preview. Press `a` to accept, `A` to accept all remaining changes this session, `r` to reject, or `e` to open the proposed content in your `$EDITOR`."
- Explain Accept All: "Pressing `A` trusts all subsequent writes for the rest of the session. This is useful after you have reviewed the first few changes and trust the agent's direction."
- Explain Edit in `$EDITOR`: "Pressing `e` opens the proposed file content in your `$EDITOR` (or `$VISUAL`). Edit it as needed and save. Clido writes the saved version instead of the agent's original proposal."

### Update: `docs/reference/flags.md`

Update `--permission-mode` entry to list `diff-review` as a valid value with a one-line description.

### Update: `docs/reference/config.md`

Update `[agent] permission_mode` entry to include `diff-review` in the list of valid values with a description.

---

## Definition of Done

- [ ] `PermGrant::EditInEditor` and `PermGrant::AllowAll` variants exist and are handled in `WriteTool` and `EditTool`.
- [ ] `PermRequest.diff` field is populated with a unified diff string before `AskUser::ask()` is called in `diff-review` mode.
- [ ] `WriteTool` in `diff-review` mode does not write any bytes to disk until `PermGrant::Allow`, `PermGrant::EditInEditor`, or `PermGrant::AllowAll` is returned.
- [ ] `EditTool` in `diff-review` mode does not modify the file until permission is granted.
- [ ] `PermGrant::Deny` returns a `ToolResult::Ok` with a user-visible rejection message (not an error that aborts the agent loop).
- [ ] `PermGrant::EditInEditor` opens `$EDITOR`, waits for exit, and writes the user-edited content.
- [ ] TUI modal renders `+` lines in green and `-` lines in red using ratatui styled spans.
- [ ] TUI modal is scrollable when the diff exceeds the visible area.
- [ ] `A` keypress in the TUI sets the `accept_all` session flag; subsequent Write/Edit calls are not blocked by a modal.
- [ ] `default` permission mode behavior is unchanged: no regression on existing tests.
- [ ] `plan-only` permission mode behavior is unchanged: no regression on existing tests.
- [ ] `diff-review` is accepted as a valid value for `--permission-mode` CLI flag.
- [ ] `diff-review` is accepted as a valid value for `[agent] permission_mode` in config.toml.
- [ ] All test cases in the test plan pass.
- [ ] `docs/guide/running-prompts.md` documents all four permission modes including `diff-review`.
- [ ] CI passes (cargo test --workspace, cargo clippy --workspace).
