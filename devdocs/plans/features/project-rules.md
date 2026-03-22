# Feature Plan: Project Rules File (CLIDO.md)

## Status

Draft — not yet scheduled for a release milestone.

---

## Problem Statement

Users working on real projects accumulate a body of conventions that should govern every agent interaction: architectural constraints ("all database access goes through the repository pattern"), style rules ("use `thiserror` for error types, never `anyhow` in library crates"), security policies ("never print secrets or API keys in output"), and team-specific norms ("follow the naming conventions in ARCHITECTURE.md"). Today, clido has no mechanism for loading these instructions automatically. The only workaround is to pass `--system-prompt-file <path>` on every invocation, which is manual, fragile, and breaks down as soon as a new team member or a CI runner runs clido without the flag.

This is a widely recognized gap. Claude Code reads `CLAUDE.md` from the project root and user home. Cursor reads `.cursorrules`. Cline reads `.clinerules`. Aider reads `.aider` conventions via command-line options stored in `.aider.conf.yml`. Every serious coding agent has solved this problem. Clido needs a first-class solution with improvements that go beyond what competitors provide.

---

## Competitive Analysis

| Tool | Rules File | Hierarchy | Merge/Override | View Active Rules |
|------|-----------|-----------|----------------|-------------------|
| Claude Code | `CLAUDE.md` | project + home | Concatenated | No slash command |
| Cursor | `.cursorrules` | Single file only | Replace | No |
| Cline | `.clinerules` | Single file only | Replace | No |
| Aider | `.aider.conf.yml` | Project only | Config values only | No |
| **Clido (proposed)** | `CLIDO.md`, `.clido/rules.md` | Project + parents + home | Concatenated with headers | `/rules` command |

Clido's differentiators:
- Full hierarchical lookup up the directory tree, not just project root and home.
- Multiple filename candidates checked in priority order.
- `[import: relative/path.md]` directive to compose rules from multiple files.
- `/rules` TUI slash command to inspect active rules without leaving the agent.
- `clido doctor` surfaces which rules files are loaded and how large they are.
- `--no-rules` and `--rules-file` for CI/scripting override.

---

## Design Decisions

### Rules File Names and Lookup Order

The following lookup is performed starting from the current working directory and walking up to the filesystem root (or user home, whichever comes first):

1. `.clido/rules.md` in the current directory
2. `CLIDO.md` in the current directory
3. Move to parent directory; repeat 1–2.
4. After the walk terminates, also check `~/.config/clido/rules.md` (global rules).

All files found are loaded. Files found closer to the project root take precedence in display order but are all included. Concatenation order: global rules first (lowest priority), then project-root rules, then subdirectory rules (closest to cwd last, highest priority / last writer wins in prompts). Each included file is wrapped with a section header in the assembled system prompt:

```
--- Rules from: /home/user/project/CLIDO.md ---
<content>
--- Rules from: /home/user/project/backend/.clido/rules.md ---
<content>
```

### Import Directive

A rules file may include other files using:

```
[import: ./conventions/naming.md]
[import: ./conventions/architecture.md]
```

Imports are resolved relative to the file containing the directive. Circular imports are detected and produce a warning (the cycle-causing file is skipped). Import depth is limited to 5 levels.

### Merge Semantics

All discovered files are concatenated — there is no override/replace behavior. The rationale: subdirectory rules are additive specializations of parent rules. If a user wants to silence a parent rule, they can add an explicit negation in the child rules file. This is consistent with how `.gitignore` works.

### Custom Rules File Path

A project can specify a non-standard rules file location in `.clido/config.toml`:

```toml
[agent]
rules_file = "docs/ai-conventions.md"
```

When `rules_file` is set, only that file is used (no hierarchical lookup). This allows teams to canonicalize the location in their repository.

### CLI Flags

| Flag | Behavior |
|------|----------|
| `--no-rules` | Skip all rules file injection for this invocation |
| `--rules-file <path>` | Use the specified file instead of the standard lookup |

`--no-rules` takes precedence over `--rules-file`. Both flags are surfaced in `clido --help` and in the CLI reference docs.

---

## Implementation Plan

### Crate: `clido-context`

**New file: `crates/clido-context/src/rules.rs`**

Responsible for:
- `RulesDiscovery` struct: holds the starting directory and configuration.
- `fn discover(cwd: &Path, config: &AgentConfig) -> Vec<RulesFile>` — performs the hierarchical scan and returns ordered list of `RulesFile` structs.
- `RulesFile { path: PathBuf, content: String, source_label: String }` — a loaded rules file.
- `fn resolve_imports(file: &RulesFile, depth: u8) -> Result<String, RulesError>` — recursively processes `[import: ...]` directives.
- `fn assemble_rules_section(files: &[RulesFile]) -> String` — concatenates with section headers into a single string ready to prepend to system prompt.

**Modify: `crates/clido-context/src/builder.rs`**

- In the context assembly pipeline, call `rules::discover()` before constructing the system prompt.
- Check `AgentConfig.no_rules` and `AgentConfig.rules_file` to apply CLI overrides.
- Prepend the assembled rules section to the system prompt string.
- Expose `assembled_rules: Vec<RulesFile>` on the `BuiltContext` struct so the TUI can read it for `/rules`.

### Crate: `clido-core`

**Modify: `crates/clido-core/src/config_loader.rs`**

- Add optional `[agent] rules_file: Option<String>` to `AgentConfig`.
- Add `no_rules: bool` (default false).

### Crate: `clido-cli`

**Modify: `crates/clido-cli/src/cli.rs`**

- Add `--no-rules` (`bool` flag) to the `RunArgs` struct.
- Add `--rules-file <PATH>` (`Option<PathBuf>`) to `RunArgs`.
- Pass both values into `AgentConfig` during setup in `agent_setup.rs`.

**Modify: `crates/clido-cli/src/tui.rs`**

- Register `/rules` as a recognized slash command.
- When `/rules` is entered, render a modal or inline panel showing:
  - Each loaded rules file path (relative to project root if possible).
  - First 3 lines of each file as a preview.
  - Total character count.
  - Message: "No rules files found." if none are loaded.
- Implement using an overlay widget in ratatui; dismiss with `Esc` or `Enter`.

### Crate: `clido-cli` — `doctor` module

**Modify: `crates/clido-cli/src/doctor.rs`**

- Add a `check_rules_files` health check that:
  - Calls `rules::discover()` with the current directory.
  - Reports each found file with path and size in bytes.
  - Warns if any file exceeds 8,000 characters (large rules files inflate every request).
  - Warns if import depth exceeds 3 levels.

---

## Config Schema

No new top-level config section required. The following keys are added to the existing `[agent]` section:

```toml
[agent]
# Override the rules file location (disables hierarchical lookup).
# Path is relative to the project root (.clido/config.toml location).
rules_file = "docs/conventions.md"

# Disable rules file injection globally (can also be set per-invocation with --no-rules).
no_rules = false
```

Global rules path: `~/.config/clido/rules.md` — no config key; presence of the file is sufficient.

---

## CLI Surface

```
clido run [--no-rules] [--rules-file <PATH>] [...]

clido doctor
  # Outputs:
  # [OK]  Rules files:
  #         /home/user/project/CLIDO.md (1,240 chars)
  #         /home/user/.config/clido/rules.md (340 chars)
```

---

## TUI Changes

### Slash Command: `/rules`

Triggered by typing `/rules` in the input box.

Display format (rendered as a floating overlay):

```
╔═══════════════════ Active Rules ═══════════════════╗
║ 1. /home/user/project/CLIDO.md (1,240 chars)       ║
║    > "All error types must implement std::error..." ║
║    > "Use tracing, not log crate for..."            ║
║                                                      ║
║ 2. /home/user/.config/clido/rules.md (340 chars)   ║
║    > "Always respond in English."                   ║
║                                                      ║
║ [Esc] dismiss                                        ║
╚══════════════════════════════════════════════════════╝
```

If no rules files are found:

```
╔═══════════════════ Active Rules ═══════════════════╗
║ No rules files found.                               ║
║                                                      ║
║ Create CLIDO.md in your project root to add rules.  ║
║ [Esc] dismiss                                        ║
╚══════════════════════════════════════════════════════╝
```

---

## Test Plan

### Unit Tests — `crates/clido-context/src/rules.rs`

1. **`test_no_rules_files_present`** — given a temporary directory with no rules files, `discover()` returns an empty vec.
2. **`test_single_clido_md_at_root`** — given a `CLIDO.md` at the project root, it is discovered and loaded.
3. **`test_dot_clido_rules_takes_priority_in_ordering`** — both `.clido/rules.md` and `CLIDO.md` exist; `.clido/rules.md` appears first in the result vec.
4. **`test_hierarchical_discovery`** — cwd is `project/src/backend/`, `CLIDO.md` is at `project/`, assert it is found.
5. **`test_global_rules_loaded`** — no project rules, but `~/.config/clido/rules.md` exists; it is included.
6. **`test_import_directive_resolved`** — a rules file contains `[import: ./sub.md]`; `sub.md` content is inlined.
7. **`test_circular_import_detected`** — file A imports file B which imports file A; second import is skipped with warning, no infinite loop.
8. **`test_import_depth_limit`** — import chain of 6 levels; depth 6 is skipped, a warning is emitted.
9. **`test_no_rules_flag_suppresses_all`** — `AgentConfig { no_rules: true }` causes `discover()` to return empty vec immediately.
10. **`test_custom_rules_file_overrides_lookup`** — `AgentConfig { rules_file: Some("custom.md") }` loads only that file.

### Integration Tests — `crates/clido-context/tests/`

11. **`test_rules_appear_in_assembled_context`** — build a full `BuiltContext` with a `CLIDO.md` present; assert the assembled system prompt contains the file's content.
12. **`test_section_headers_present`** — assembled system prompt contains `--- Rules from:` headers with correct file paths.

### TUI Tests — `crates/clido-cli/src/tui.rs`

13. **`test_rules_slash_command_recognized`** — entering `/rules` is parsed as the rules command, not sent to the agent.
14. **`test_rules_overlay_shows_files`** — given a mocked `BuiltContext` with two rules files, the overlay widget renders both paths.

---

## Docs Updates

- **New file**: `docs/guide/project-rules.md` — explains the concept, shows a sample `CLIDO.md`, covers hierarchical lookup, import directive, CLI flags, and slash command.
- **Update**: `docs/reference/configuration.md` — add `[agent] rules_file` and `no_rules` entries.
- **Update**: `docs/reference/slash-commands.md` — add `/rules` entry with description and screenshot.
- **Update**: `docs/guide/getting-started.md` — mention creating a `CLIDO.md` as a recommended first step.
- **Update**: `docs/reference/cli-flags.md` — add `--no-rules` and `--rules-file` entries.

---

## Definition of Done

- [ ] `RulesDiscovery::discover()` correctly walks from cwd to root, collecting `.clido/rules.md` and `CLIDO.md` at each level, then appends the global `~/.config/clido/rules.md`.
- [ ] `[import: path]` directives are resolved recursively up to depth 5; circular imports are detected and warned; the import chain never panics.
- [ ] The assembled rules section is prepended to the system prompt in `clido-context`'s builder before any tool call is made.
- [ ] `--no-rules` flag fully suppresses rules injection; no rules content appears in the assembled system prompt.
- [ ] `--rules-file <path>` loads exactly the specified file and skips hierarchical lookup.
- [ ] `[agent] rules_file` config key works identically to `--rules-file` when set in `.clido/config.toml`.
- [ ] `/rules` TUI slash command renders a dismissible overlay listing all active rules files with path, size, and preview lines.
- [ ] `clido doctor` output includes a `Rules files` section listing each discovered file, its size, and a warning if any file exceeds 8,000 characters.
- [ ] All 14 tests listed in the test plan pass with `cargo test`.
- [ ] No rules-related code introduces an `unwrap()` or `expect()` that can panic on invalid user input; all errors surface as user-visible messages.
- [ ] `docs/guide/project-rules.md` is written and linked from the VitePress sidebar.
- [ ] Configuration reference and CLI flags reference are updated.
- [ ] Slash commands reference is updated with `/rules`.
