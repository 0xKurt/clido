# Feature Plan: Native Git Awareness

**Status:** Planned
**Target release:** V2
**Crates affected:** `clido-context` (new), `clido-tools`, `clido-cli`, `clido-agent`, `clido-core`
**Docs affected:** `docs/guide/git.md` (new), `docs/guide/running-prompts.md`, `docs/reference/flags.md`, `docs/reference/config.md`

---

## Problem Statement

Clido can execute arbitrary `git` commands through the Bash tool, but it has no native, structured understanding of git state at session start. The agent must discover the current branch, what is staged, and recent history through shell invocations during the conversation — wasting tokens, introducing latency, and making it easy to produce inaccurate commit messages or miss relevant staged hunks.

Compared to established tools:

- **Aider** was built around git from day one. It auto-stages, auto-commits, and has first-class git undo. The agent always knows the diff it is about to commit.
- **Claude Code** reads `git status` and recent log automatically and injects them as context when the working directory is a git repository. This is surfaced in the thinking block so the model already knows what is staged before the user types anything.
- **Cline** does not inject git context but shows the diff of every write before applying it. Combined with file-level undo, the user always has a safe rollback path.

Clido's current state: none of these. The agent treats git as an opaque CLI tool. If the user says "commit this change with a descriptive message", the agent must shell out to `git diff --staged`, parse the output itself, and write a commit message with no structured prompt support. This leads to:

1. Token waste on repeated shell reads of git state.
2. No validation that write operations left the working tree in a clean, intentional state.
3. No `clido commit` shortcut for the most common daily workflow.
4. The TUI shows no git context — the user does not know what branch they are on without leaving the terminal.

---

## Our Improvements Over Competitors

| Capability | Aider | Claude Code | Clido (after this feature) |
|---|---|---|---|
| Git context in system prompt | No (uses git in-process) | Yes (status + log) | Yes (`<git-context>` block, status + log + staged diff) |
| Safe read-only git tool | No | No (uses Bash) | Yes (`GitTool` with subcommand allowlist) |
| Auto-commit after task | Yes (default on) | No | Yes (default off, opt-in) |
| AI commit message generation | Yes | Via `clide commit` | Yes (`clido commit` subcommand) |
| TUI git status slash command | No | No | Yes (`/git` inline status) |
| Conventional commits style | No | No | Yes (configurable) |

Key differentiator: `GitTool` exposes only safe, read-only subcommands by default. Write operations (add, commit, stash push) are gated behind explicit user approval in Default permission mode. This matches the principle of least surprise — the agent can read git freely but cannot change git history without the user knowing.

---

## Design Decisions and Rationale

### New crate: `clido-context`

Git context injection is not a "tool" the agent calls — it is ambient context injected once at session start. Placing it in `clido-tools` would conflate two concepts. A new `clido-context` crate holds all session-start context providers: git context today, potentially IDE diagnostics, environment variables, or language server info in the future. This keeps `clido-agent` clean and makes context providers independently testable.

### `<git-context>` XML block vs YAML frontmatter

XML blocks are what Claude models already see in official tooling (Claude Code uses XML for tool results). Using `<git-context>` is consistent with existing prompt structure in `clido-agent` and is easier to strip from logs than YAML frontmatter mixed into the system prompt.

### Read-only `GitTool` by default

Aider's aggressive auto-staging was controversial. Many users reported it committing partial work or unintentional files. Clido defaults to read-only `GitTool`. Write-mode git operations (add, commit, stash) require `permission_mode = "permissive"` or explicit user confirmation in Default mode. This is opt-in safety rather than opt-out.

### Auto-commit default off

Auto-commit is powerful but dangerous for users who manage commits carefully (e.g., squash-merge workflows, signed commits, DCO). It defaults to `false`. When enabled, the agent always shows a diff + proposed message and waits for confirmation before committing.

### `clido commit` as a separate subcommand

This is a user-facing command, not agent-driven. It is the fastest path from "I just edited code" to "it is committed with a good message." It runs outside an agent session: reads `git diff --staged`, sends to LLM, returns a message. No tool calls, no agent loop overhead.

---

## Architecture

### `clido-context` crate

```
crates/clido-context/
  src/
    lib.rs          -- pub trait ContextProvider; pub fn collect_context()
    git.rs          -- GitContext struct and builder
    env.rs          -- (future) env/shell context
  Cargo.toml
```

`GitContext` in `git.rs`:

```rust
pub struct GitContext {
    pub branch: Option<String>,
    pub status_short: Option<String>,   // output of git status --short
    pub log_oneline: Option<String>,    // last 5 commits, git log --oneline -5
    pub staged_diff_stat: Option<String>, // git diff --stat --staged
}

impl GitContext {
    pub fn collect(root: &Path) -> Self { ... }
    pub fn to_prompt_block(&self) -> Option<String> { ... }
}
```

`collect()` uses `std::process::Command` to run each git command. If the working directory is not a git repo (exit code non-zero on `git rev-parse --is-inside-work-tree`), all fields are `None` and `to_prompt_block()` returns `None`. The system prompt injection step in `clido-agent/src/agent_setup.rs` calls `GitContext::collect()` and appends the block if `Some`.

`to_prompt_block()` renders:

```xml
<git-context>
<branch>main</branch>
<status>
M  src/lib.rs
?? scratch.txt
</status>
<recent-commits>
abc1234 Fix session serialization
def5678 Add Grep tool
</recent-commits>
<staged-diff-stat>
 src/lib.rs | 12 +++++++-----
 1 file changed, 7 insertions(+), 5 deletions(-)
</staged-diff-stat>
</git-context>
```

### `GitTool` in `clido-tools`

New file: `crates/clido-tools/src/git.rs`

Implements the `Tool` trait. Input schema:

```json
{
  "subcommand": "status|diff|log|branch|show|stash-list",
  "path": "(optional) file path for diff",
  "count": "(optional) integer for log -n"
}
```

Subcommand dispatch:

| Subcommand | Shell equivalent | Notes |
|---|---|---|
| `status` | `git status --short` | Always safe |
| `diff` | `git diff [-- path]` | Shows unstaged changes |
| `diff-staged` | `git diff --staged [-- path]` | Shows staged changes |
| `log` | `git log --oneline -N` | N defaults to 10, max 50 |
| `branch` | `git branch --show-current` | Current branch only |
| `show` | `git show --stat HEAD` | Last commit summary |
| `stash-list` | `git stash list` | Read only |

Write-mode subcommands (`add`, `commit`, `stash-push`) are not in the allowed list. If the agent attempts to use Bash to run `git commit`, it is not blocked — but the `GitTool` itself never commits. This maintains the read-only contract for structured tool calls.

`GitTool` uses `crate::sandbox::run_command_in_root()` (already used by `BashTool`) so it respects the working directory and timeout limits.

### Auto-commit flow

Location: `crates/clido-agent/src/agent_loop.rs`, after `AgentLoop::run()` completes successfully.

```
1. Check config: [git] auto_commit = true
2. Run git status --porcelain. If empty, skip.
3. Run git diff HEAD to get full diff.
4. Send diff to LLM with prompt: "Write a git commit message for this diff. Use {commit_style} style."
5. Display diff + proposed message to user via TUI confirm modal.
6. User: Accept → git add -A && git commit -m "{message}"
         Edit   → open $EDITOR on message, re-read, git add -A && git commit -m "{edited}"
         Cancel → skip commit, log skipped
```

This flow is implemented as `AutoCommit::run(config, event_emitter, session)` in `crates/clido-agent/src/auto_commit.rs` (new file).

### `clido commit` subcommand

Location: new match arm in `crates/clido-cli/src/cli.rs`.

```rust
Commands::Commit { dry_run } => {
    commit::run(config, dry_run).await?;
}
```

New file: `crates/clido-cli/src/commit.rs`

Steps:

1. Run `git diff --staged`. If empty, print "Nothing staged. Stage changes with git add first." and exit 1.
2. Build a minimal LLM prompt (no tools, no agent loop): send diff as user message, system prompt instructs to write a conventional-commits message.
3. Stream response to terminal.
4. Show preview: the staged diff (condensed) and the proposed message.
5. Prompt: `[A]ccept [E]dit [C]ancel` (using `inquire` as already in the codebase).
6. On Accept: run `git commit -m "{message}"`, print the commit hash.
7. On Edit: open `$EDITOR` on a temp file containing the message, re-read, run `git commit -m "{edited}"`.
8. On Cancel: exit 0 with message "Commit cancelled."

### TUI slash command `/git`

Location: `crates/clido-cli/src/tui.rs`, in the slash command handler.

When the user types `/git` and presses Enter:

1. Run `git status --short` and `git branch --show-current`.
2. Render output as a `ChatLine::SystemMessage` block (styled differently from tool output).
3. Show branch name, changed file list, staged file count, untracked file count.

The `/git` command does not call the agent — it is a local shell invocation inside the TUI event loop, similar to how `/clear` and `/help` are handled.

---

## Configuration Schema

In `config.toml`:

```toml
[git]
# Inject git context block into system prompt at session start.
context_in_prompt = true

# Automatically create a commit after the agent completes a task and the
# working tree has changed. Requires user confirmation before committing.
auto_commit = false

# Commit message style used for both auto-commit and `clido commit`.
# "conventional" = Conventional Commits (feat:, fix:, chore:, etc.)
# "descriptive"  = Free-form, multi-sentence description
commit_style = "conventional"
```

Config keys are added to `AgentConfig` in `crates/clido-core/src/lib.rs` and loaded in `crates/clido-core/src/config_loader.rs`.

---

## CLI Surface

### New flag

```
clido run --no-git-context "..."
```

`--no-git-context` suppresses git context injection for this session even if `context_in_prompt = true` in config. Useful when the user is in a large monorepo and git commands are slow.

### New subcommand

```
clido commit [--dry-run]
```

`--dry-run`: generate and print the commit message but do not run `git commit`. Exits 0.

### Updated help text

`clido --help` lists `commit` in the subcommand table.

---

## TUI Changes

| Location | Change |
|---|---|
| `tui.rs` slash command list | Add `/git` with description "Show current git status" |
| `/git` handler | Run `git status --short` + `git branch --show-current`, emit `ChatLine::SystemMessage` |
| Permission modal | No changes for Phase 1; auto-commit uses existing confirm dialog |

---

## Implementation Steps (ordered)

1. Create `crates/clido-context/` crate skeleton (Cargo.toml, src/lib.rs, src/git.rs).
2. Implement `GitContext::collect()` with graceful fallback when not a git repo.
3. Implement `GitContext::to_prompt_block()`.
4. Wire `GitContext` into `agent_setup.rs`: append block to system prompt if `context_in_prompt = true`.
5. Add `[git]` section to `AgentConfig` and `config_loader.rs`.
6. Implement `GitTool` in `crates/clido-tools/src/git.rs` with read-only subcommands.
7. Register `GitTool` in the tool registry in `crates/clido-tools/src/lib.rs`.
8. Implement `crates/clido-cli/src/commit.rs` (`clido commit` subcommand).
9. Add `Commands::Commit` variant to `cli.rs` and wire to `commit::run()`.
10. Implement `crates/clido-agent/src/auto_commit.rs`.
11. Wire `AutoCommit::run()` into `AgentLoop::run()` post-completion hook.
12. Add `/git` slash command handler in `tui.rs`.
13. Add `--no-git-context` flag to `RunArgs` in `cli.rs`.
14. Write all tests (see Test Plan below).
15. Write `docs/guide/git.md`.
16. Update `docs/guide/running-prompts.md`, `docs/reference/flags.md`, `docs/reference/config.md`.

---

## Test Plan

All tests live in the respective crate's `tests/` directory or in `#[cfg(test)]` modules.

### Unit tests: `clido-context`

**`test_git_context_collects_branch`**
Set up a temp directory, `git init`, `git commit --allow-empty`. Assert `GitContext::collect()` returns a `branch` that is `"main"` or `"master"`.

**`test_git_context_graceful_not_a_repo`**
Call `GitContext::collect()` on a temp directory with no `.git`. Assert all fields are `None` and `to_prompt_block()` returns `None`.

**`test_git_context_status_short_shows_modified`**
Create a temp git repo, write a file, stage it. Assert `status_short` contains `"A "` prefix.

**`test_git_context_prompt_block_format`**
Call `to_prompt_block()` on a `GitContext` with all fields populated. Assert output starts with `<git-context>` and ends with `</git-context>`, and contains `<branch>`, `<status>`, `<recent-commits>`.

**`test_git_context_skipped_when_config_disabled`**
Set `context_in_prompt = false` in config. Assert the system prompt built by `agent_setup` does not contain `<git-context>`.

### Unit tests: `clido-tools` (GitTool)

**`test_git_tool_status_returns_output`**
Call `GitTool` with `subcommand = "status"` in a temp git repo. Assert result is `ToolResult::Ok` and contains expected status text.

**`test_git_tool_diff_returns_diff`**
Modify a tracked file, call `GitTool` with `subcommand = "diff"`. Assert result contains `@@` diff markers.

**`test_git_tool_log_returns_commits`**
Create two commits in a temp repo. Call `GitTool` with `subcommand = "log"`. Assert result contains two SHA lines.

**`test_git_tool_log_count_capped_at_50`**
Call `GitTool` with `subcommand = "log"` and `count = 9999`. Assert the command run is `git log --oneline -50` (capped).

**`test_git_tool_unknown_subcommand_returns_error`**
Call `GitTool` with `subcommand = "push"`. Assert result is `ToolResult::Err` with a message about unsupported subcommand.

**`test_git_tool_not_in_repo_returns_error`**
Call `GitTool` in a non-git directory. Assert result is `ToolResult::Err` with a message about not being in a git repository.

### Unit tests: `clido-cli` (commit subcommand)

**`test_commit_exits_if_nothing_staged`**
Mock `git diff --staged` to return empty. Assert `commit::run()` returns `Err` with "Nothing staged" message.

**`test_commit_dry_run_does_not_call_git_commit`**
Run `commit::run(config, dry_run: true)` with a mock staged diff. Assert `git commit` was never invoked.

**`test_commit_conventional_style_prompt`**
Assert the LLM prompt sent during `commit::run()` contains the string `"Conventional Commits"` when `commit_style = "conventional"`.

### Integration tests: auto-commit

**`test_auto_commit_skipped_when_disabled`**
Run agent loop with `auto_commit = false`, modify a file. Assert no `git commit` process was spawned.

**`test_auto_commit_skipped_when_no_changes`**
Run agent loop with `auto_commit = true`, no file changes. Assert no commit prompt is shown.

**`test_auto_commit_shows_preview_and_commits_on_accept`**
Run agent loop with `auto_commit = true`, mock LLM commit message, simulate user pressing Accept. Assert `git log -1` shows the expected commit.

**`test_auto_commit_cancel_leaves_working_tree_unchanged`**
Simulate user pressing Cancel on the commit preview. Assert working tree has uncommitted changes.

---

## Docs Pages

### New: `docs/guide/git.md`

Covers: git context injection, `GitTool` subcommands, `clido commit` workflow, auto-commit setup, `/git` slash command, config reference for `[git]` section.

### Update: `docs/guide/running-prompts.md`

Add section: "Git-aware sessions" explaining that git context is injected automatically, and how to disable it with `--no-git-context`.

### Update: `docs/reference/flags.md`

Add `--no-git-context` to the flags table.
Add `commit [--dry-run]` to the subcommands table.

### Update: `docs/reference/config.md`

Add `[git]` section with all three keys, types, defaults, and examples.

---

## Definition of Done

- [ ] `clido-context` crate builds and all unit tests pass.
- [ ] `GitContext::collect()` returns `None` fields gracefully when not in a git repo; no panic, no unwrap.
- [ ] System prompt contains `<git-context>` block when running in a git repo with default config.
- [ ] `--no-git-context` flag suppresses the block without error.
- [ ] `GitTool` is registered in the tool registry and appears in `clido tools list` output.
- [ ] `GitTool` returns an error (not a panic) when called with any write-mode subcommand.
- [ ] `clido commit` generates a commit message and prompts for confirmation before running `git commit`.
- [ ] `clido commit --dry-run` prints message and exits without running `git commit`.
- [ ] `clido commit` exits with a clear error message when nothing is staged.
- [ ] Auto-commit is disabled by default (`auto_commit = false`); enabling it requires explicit config change.
- [ ] Auto-commit shows diff + proposed message preview before committing; no silent commits.
- [ ] `/git` slash command in TUI shows branch and status without starting an agent turn.
- [ ] `[git]` config section is documented in `docs/reference/config.md`.
- [ ] All new code has no `unwrap()` on paths reachable at runtime in production builds.
- [ ] CI passes (cargo test --workspace, cargo clippy --workspace).
