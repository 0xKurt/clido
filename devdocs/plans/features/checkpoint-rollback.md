# Feature Plan: Checkpoint and Rollback

## Status

Draft — not yet scheduled for a release milestone.

---

## Problem Statement

When a coding agent makes a bad series of edits — overwrites a file with wrong content, introduces a cascade of broken imports, or runs a `sed` one-liner that corrupts a configuration file — recovery is painful. The user must figure out which files were changed, find the original content, and restore everything manually. Users who have a git repository in good shape can use `git diff` and `git checkout`; but they still need to identify the right files, know which commit to roll back to, and execute the right commands without making things worse. Users without a git repository (common in early-stage projects, throwaway scripts, or when the agent is working on a non-git directory) have no safety net at all.

Even with git, the current experience requires context-switching out of the agent entirely. This breaks flow and forces the user to reason about git state while also reasoning about the agent task.

---

## Competitive Analysis

| Tool | Auto Snapshot | Named Checkpoints | Rollback Command | Diff View | Session-Scoped |
|------|--------------|-------------------|-----------------|-----------|----------------|
| Claude Code | Snapshot before each file operation | No | No CLI command | No | No (global) |
| Cursor | Undo/redo in editor | No | No | Editor built-in | N/A |
| Cline | No built-in | No | No | No | No |
| Aider | Relies on git | No | `--no-dirty-repo` guard only | Via `git diff` | No |
| **Clido (proposed)** | Auto-checkpoint per turn | Yes (`/checkpoint save`) | `clido rollback` + `/rollback` | Built-in diff view | Yes (per-session) |

Clido's differentiators:
- Auto-checkpoint is taken at the **turn level** (before a turn that results in any file modification), not at the individual file operation level. This means one checkpoint captures all files in a coherent pre-turn state, which is the unit users actually want to roll back.
- Named checkpoints let users mark "before big refactor" before intentionally asking the agent to do something risky.
- `clido rollback` is a first-class CLI command with an interactive file picker (not just a bare restore).
- Diff view shows exactly what changed since a checkpoint before the user commits to rolling back.
- Checkpoints are stored per-session, so they do not pollute other sessions or the global git state.
- Works whether or not the project is a git repository.
- Session fork inherits checkpoint history, enabling a "try this direction" workflow.

---

## Design Decisions

### Auto-Checkpoint Trigger

An auto-checkpoint is taken at the start of a user turn if and only if that turn results in at least one call to a file-mutating tool. File-mutating tools are: `Write`, `Edit`, and `Bash`. The checkpoint is created **before** any tool is executed in the turn — this means it captures the state just before the agent touches anything.

The trigger is evaluated lazily: the checkpoint is created when the first file-mutating tool is about to be called, not at the start of every turn. This avoids creating empty checkpoints for read-only turns.

This behavior is controlled by `[agent] auto_checkpoint = true` (default). Setting it to `false` disables auto-checkpointing; users can still create manual checkpoints.

### Checkpoint Storage Layout

```
.clido/checkpoints/
  <session-id>/
    <checkpoint-id>/
      manifest.json
      files/
        <content-hash-1>   — raw file bytes (content-addressed)
        <content-hash-2>
        ...
```

`manifest.json` schema:
```json
{
  "checkpoint_id": "ck_01j9xz...",
  "session_id": "sess_01j9...",
  "created_at": "2026-03-21T14:32:00Z",
  "name": "before big refactor",
  "auto": true,
  "turn_index": 7,
  "files": [
    {
      "path": "/Users/kurt/project/src/main.rs",
      "content_hash": "sha256:abc123...",
      "size_bytes": 4210
    }
  ]
}
```

Content-addressing (`sha256` of the file bytes) means identical files in different checkpoints share the same storage blob. This keeps storage size manageable for projects with many checkpoints.

### What Files Are Snapshotted

At auto-checkpoint time, the set of files to snapshot is determined by tracking which files were modified in the **previous** turn (i.e., what was changed last time). At the start of each new turn, these tracked paths from the prior turn are snapshotted before the new turn's tools run.

For the very first file-mutating turn in a session, the snapshot is taken of the files that are about to be written/edited (their current on-disk state at the moment of first tool call).

For manual `/checkpoint save` commands, the user may optionally specify files; if none are specified, all files modified in the current session so far are included.

### Rollback Behavior

Rolling back a checkpoint restores all files listed in the checkpoint's manifest to their snapshotted content. The rollback is atomic at the filesystem level using write-to-temp-file-then-rename semantics to avoid partial restores.

After rollback, a new auto-checkpoint is created capturing the post-rollback state (so the rollback itself is undoable).

Rollback is idempotent: rolling back to the same checkpoint twice produces the same result.

### Session Fork Integration

When a session is forked (`clido sessions fork <session-id>`), the new session inherits:
- The full checkpoint history directory (copied, not shared, to keep sessions independent).
- The turn index counter continues from the fork point.

### Cleanup

Checkpoint data can grow large on long-running sessions. The following cleanup mechanisms are provided:
- `clido sessions clear-checkpoints [--older-than <days>]` removes checkpoint directories for sessions older than N days (default 30).
- `clido sessions clear-checkpoints --session <session-id>` removes checkpoints for a specific session.
- Checkpoints are never automatically deleted within an active session.

---

## Implementation Plan

### New Crate: `clido-checkpoint`

Create `crates/clido-checkpoint/` with the following structure:

```
crates/clido-checkpoint/
  Cargo.toml
  src/
    lib.rs           — pub mod checkpoint; pub mod restore; pub mod diff; pub mod storage;
    checkpoint.rs    — CheckpointManager, create_checkpoint, list_checkpoints
    restore.rs       — restore_checkpoint, RestoreResult
    diff.rs          — compute_diff_since_checkpoint
    storage.rs       — content-addressed file storage, manifest read/write
```

`Cargo.toml` dependencies:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
tokio = { version = "1", features = ["fs"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v7"] }
clido-core = { path = "../clido-core" }
```

### `crates/clido-checkpoint/src/checkpoint.rs`

- `pub struct CheckpointManager { session_id: String, base_dir: PathBuf, config: CheckpointConfig }`
- `pub async fn create_checkpoint(&self, files: &[PathBuf], name: Option<&str>, auto: bool) -> Result<CheckpointId, CheckpointError>` — snapshots the given files, writes manifest.
- `pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointMeta>, CheckpointError>` — reads all manifests in the session directory, sorted by `created_at` descending.
- `pub async fn get_checkpoint(&self, id: &CheckpointId) -> Result<Checkpoint, CheckpointError>` — loads a full checkpoint including file contents.
- Internal helper: `fn files_modified_in_session(&self) -> Vec<PathBuf>` — returns the union of all files touched by any tool call in the current session (tracked via `SessionFileTracker`).

### `crates/clido-checkpoint/src/restore.rs`

- `pub struct RestoreResult { restored_files: Vec<PathBuf>, skipped_files: Vec<(PathBuf, String)> }`
- `pub async fn restore_checkpoint(checkpoint: &Checkpoint, dry_run: bool) -> Result<RestoreResult, CheckpointError>` — restores each file using write-to-temp-then-rename.
- `dry_run = true` returns what would be restored without writing to disk (used by the diff/confirm flow).

### `crates/clido-checkpoint/src/diff.rs`

- `pub struct FileDiff { path: PathBuf, before: String, after: String, unified_diff: String }`
- `pub async fn diff_since_checkpoint(checkpoint: &Checkpoint) -> Result<Vec<FileDiff>, CheckpointError>` — for each file in the checkpoint, reads the current on-disk version and computes a unified diff against the snapshotted version.
- Uses the `similar` crate for diff computation.

### `crates/clido-checkpoint/src/storage.rs`

- `pub fn store_file_content(base_dir: &Path, content: &[u8]) -> Result<ContentHash, StorageError>` — sha256 hash the content, write to `base_dir/<hash>` if not already present.
- `pub fn load_file_content(base_dir: &Path, hash: &ContentHash) -> Result<Vec<u8>, StorageError>` — read and return content for a given hash.
- `pub fn write_manifest(checkpoint_dir: &Path, manifest: &CheckpointManifest) -> Result<(), StorageError>`
- `pub fn read_manifest(checkpoint_dir: &Path) -> Result<CheckpointManifest, StorageError>`

### `SessionFileTracker`

Tracking which files have been touched across a session is needed for the "snapshot files modified in this session" feature of manual checkpoints. This is maintained in `clido-agent`:

**Modify: `crates/clido-agent/src/agent_loop.rs`**

- Add `session_file_tracker: SessionFileTracker` to `AgentLoop` state.
- After each `Write`, `Edit`, or `Bash` tool call, record the affected file paths in the tracker.
- For `Bash`, affected files are not known statically — the tracker records all files that existed before the call and changed after (using a pre/post filesystem snapshot of the working directory). This is best-effort and limited to files within the project working directory.
- Expose `tracker.touched_files() -> &[PathBuf]` for use by `CheckpointManager`.

**Auto-checkpoint trigger in `agent_loop.rs`**:

```rust
// Before executing the first file-mutating tool in a turn:
if self.config.auto_checkpoint && !self.checkpoint_taken_this_turn {
    let files_to_snapshot = self.session_file_tracker.touched_files_previous_turns();
    // For the first turn, snapshot the files about to be written.
    if !files_to_snapshot.is_empty() {
        self.checkpoint_manager.create_checkpoint(&files_to_snapshot, None, true).await?;
    }
    self.checkpoint_taken_this_turn = true;
}
```

Reset `checkpoint_taken_this_turn = false` at the start of each new turn.

### Crate: `clido-core`

**Modify: `crates/clido-core/src/config_loader.rs`**

```rust
#[derive(Debug, Deserialize)]
pub struct CheckpointConfig {
    /// Enable automatic checkpointing before file-mutating turns. Default: true.
    pub auto_checkpoint: bool,
    /// Max number of checkpoints to retain per session. 0 = unlimited. Default: 50.
    pub max_checkpoints_per_session: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self { auto_checkpoint: true, max_checkpoints_per_session: 50 }
    }
}
```

### Crate: `clido-cli`

**Modify: `crates/clido-cli/src/cli.rs`**

Add subcommand:
```
clido checkpoint save [--name <NAME>] [<FILES>...]
  # Creates a named manual checkpoint.
  # If no files are specified, snapshots all files modified in the current session.

clido rollback [--session <SESSION_ID>] [<CHECKPOINT_ID>]
  # If CHECKPOINT_ID is omitted, shows an interactive picker.
  # Shows a diff view before asking for confirmation.
  # --session selects the session; defaults to the most recent session.
```

Add `sessions` subcommand entries:
```
clido sessions clear-checkpoints [--older-than <DAYS>] [--session <SESSION_ID>]
```

**Modify: `crates/clido-cli/src/tui.rs`**

Register `/checkpoint` and `/rollback` as slash commands.

`/checkpoint` behavior:
- Entering `/checkpoint` alone creates an auto-named checkpoint with timestamp.
- Entering `/checkpoint save <name>` creates a named checkpoint.
- Confirmation message appears in the chat: `Checkpoint saved: ck_01j9xz (3 files)`.

`/rollback` behavior:
1. Display a scrollable list of checkpoints for the current session, showing:
   - Timestamp (relative: "2 minutes ago")
   - Name (or "auto" for auto-checkpoints)
   - Number of files in snapshot
   - Turn index
2. User selects a checkpoint with arrow keys + Enter.
3. Show a diff view: for each snapshotted file, show a unified diff of current state vs. checkpoint state.
4. Prompt: `Restore 3 files to checkpoint state? [y/N]`
5. On confirm: perform restore, show `Rolled back to checkpoint ck_01j9xz (3 files restored)`.
6. On cancel: return to normal TUI state without changes.

Dismiss at any point with `Esc`.

---

## Config Schema

```toml
[agent]
# Enable automatic checkpoint before file-mutating agent turns. Default: true.
auto_checkpoint = true

# Maximum number of checkpoints retained per session.
# When the limit is reached, the oldest auto-checkpoint is deleted.
# Named checkpoints are never automatically deleted. Default: 50.
max_checkpoints_per_session = 50
```

---

## CLI Surface

```
clido checkpoint save [--name <NAME>] [<FILES>...]
clido rollback [--session <SESSION_ID>] [<CHECKPOINT_ID>]
clido sessions clear-checkpoints [--older-than <DAYS>] [--session <SESSION_ID>]
```

`clido rollback` with no arguments opens an interactive picker using the `inquire` crate (consistent with first-run setup). The picker shows all checkpoints for the most recent session, with name, timestamp, and file count.

---

## TUI Changes

### Slash Command: `/checkpoint`

- `/checkpoint` — create checkpoint with auto-generated name.
- `/checkpoint save <name>` — create named checkpoint.

Feedback in chat area (not a modal, just an inline message):
```
[checkpoint] Saved: "before big refactor" (5 files, ck_01j9xz)
```

### Slash Command: `/rollback`

Opens a full-screen modal overlay:

```
╔═══════════════════ Rollback ═══════════════════════╗
║ Select a checkpoint to restore:                      ║
║                                                      ║
║ ▶ 2 minutes ago  [auto]           3 files  ck_01j9  ║
║   14 minutes ago "before refact"  5 files  ck_01j8  ║
║   1 hour ago     [auto]           1 file   ck_01j7  ║
║                                                      ║
║ [↑↓] navigate  [Enter] select  [Esc] cancel          ║
╚══════════════════════════════════════════════════════╝
```

After selecting, diff view:

```
╔═══════════════ Diff: ck_01j9xz ════════════════════╗
║ src/main.rs                                          ║
║ @@ -10,7 +10,7 @@                                   ║
║ -    let config = Config::load()?;                  ║
║ +    let config = Config::default();                 ║
║                                                      ║
║ src/lib.rs (no changes)                             ║
║                                                      ║
║ Restore 2 files? [y/N] _                            ║
╚══════════════════════════════════════════════════════╝
```

---

## Test Plan

### Unit Tests — `crates/clido-checkpoint/src/`

1. **`test_create_checkpoint_snapshots_files`** — create a temp dir with two files, call `create_checkpoint`; assert manifest contains both file paths and content hashes, assert blobs exist on disk.
2. **`test_content_addressing_deduplication`** — two checkpoints include the same file content; assert only one blob is stored.
3. **`test_restore_checkpoint_writes_files`** — create checkpoint, modify files, call `restore_checkpoint`; assert files match original content.
4. **`test_restore_is_idempotent`** — call `restore_checkpoint` twice in succession; second call produces same result as first, no errors.
5. **`test_restore_dry_run_does_not_write`** — `restore_checkpoint(checkpoint, dry_run: true)`; assert files on disk are unchanged after the call.
6. **`test_diff_since_checkpoint_detects_changes`** — create checkpoint, modify one file; `diff_since_checkpoint` returns one `FileDiff` with non-empty `unified_diff`.
7. **`test_diff_since_checkpoint_no_changes`** — create checkpoint, do not modify files; `diff_since_checkpoint` returns empty vec.
8. **`test_list_checkpoints_sorted_by_time`** — create 3 checkpoints in sequence; `list_checkpoints` returns them newest-first.
9. **`test_max_checkpoints_per_session_evicts_oldest_auto`** — set `max_checkpoints_per_session = 3`, create 4 auto-checkpoints; oldest is removed, named checkpoints are preserved.

### Integration Tests — `crates/clido-agent/tests/`

10. **`test_auto_checkpoint_triggered_before_write`** — run a mock agent turn that calls `Write`; assert a checkpoint exists in `.clido/checkpoints/<session-id>/` before the write resolves.
11. **`test_auto_checkpoint_not_triggered_for_readonly_turn`** — run a mock agent turn that calls only `ReadFile` and `Glob`; assert no checkpoint is created.
12. **`test_rollback_multi_file_edit`** — agent modifies 3 files in one turn; checkpoint is created; restore checkpoint; assert all 3 files are restored to original content.
13. **`test_rollback_after_second_turn`** — two turns, each modifying different files; roll back to checkpoint before first turn; assert both turns' changes are reverted.

### TUI Tests — `crates/clido-cli/src/tui.rs`

14. **`test_checkpoint_slash_command_recognized`** — entering `/checkpoint` is parsed as checkpoint command, not sent to agent.
15. **`test_rollback_slash_command_opens_picker`** — entering `/rollback` renders the checkpoint picker overlay.

---

## Docs Updates

- **New file**: `docs/guide/checkpoint-rollback.md` — explains auto-checkpoint behavior, how to create named checkpoints, how to use `clido rollback` and `/rollback`, storage layout, and session cleanup.
- **Update**: `docs/guide/sessions.md` — add section on checkpoint inheritance when forking sessions; add `clear-checkpoints` command.
- **Update**: `docs/reference/key-bindings.md` — add `/checkpoint` and `/rollback` slash command entries.
- **Update**: `docs/reference/configuration.md` — add `[agent] auto_checkpoint` and `max_checkpoints_per_session` entries.
- **Update**: `docs/reference/cli-flags.md` — add `clido checkpoint` and `clido rollback` subcommand entries.
- **Update**: `docs/developer/architecture.md` — mention `clido-checkpoint` crate and `SessionFileTracker`.

---

## Definition of Done

- [ ] `clido-checkpoint` crate builds cleanly; `cargo clippy -- -D warnings` passes with no warnings.
- [ ] Auto-checkpoint is created before the first file-mutating tool call in each turn when `auto_checkpoint = true`; no checkpoint is created for read-only turns.
- [ ] `[agent] auto_checkpoint = false` disables auto-checkpointing; manual `/checkpoint` still works.
- [ ] Content-addressed storage deduplicates identical file content across checkpoints; verified by checking that a second checkpoint of an unchanged file does not create a new blob.
- [ ] `restore_checkpoint` restores all snapshotted files atomically (write-temp-then-rename); a partial failure leaves previously-restored files in their restored state and reports the failing file.
- [ ] Rollback is idempotent: running `clido rollback <same-id>` twice produces the same filesystem state with no error on the second run.
- [ ] `/rollback` TUI command shows a picker, then a diff view, then a confirmation prompt; user can cancel at any step without modifying files.
- [ ] `clido rollback` CLI command works non-interactively when a `<CHECKPOINT_ID>` is provided (no picker shown); shows diff and prompts for `[y/N]` confirmation.
- [ ] `clido sessions clear-checkpoints --older-than 30` removes checkpoint directories for sessions whose last modification time is more than 30 days ago.
- [ ] Forked sessions inherit the parent session's checkpoint history (directory is copied at fork time).
- [ ] `max_checkpoints_per_session` eviction deletes the oldest auto-checkpoint when the limit is exceeded; named checkpoints are never auto-deleted.
- [ ] All 15 tests listed in the test plan pass with `cargo test`.
- [ ] `docs/guide/checkpoint-rollback.md` is written and linked from the VitePress sidebar.
- [ ] Sessions guide, key-bindings reference, configuration reference, and CLI flags reference are all updated.
