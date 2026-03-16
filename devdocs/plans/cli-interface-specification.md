# Clido CLI Interface Specification

This document is the canonical product specification for Clido's command-line interface. It defines every user-facing interaction across all releases. Implementation must follow this spec; the main roadmap ([development-plan.md](development-plan.md)) and release plans reference it.

**Evidence base:** This spec is grounded in [REPORT.md](../REPORT.md) and [ARTIFACTS.md](../ARTIFACTS.md) (reverse-engineering of Claude CLI and Cursor agent).

---

## 0. Design Principles and Non-Goals

### Principles

- **CLI-first, not API-first** — Clido is a tool users run in a terminal; automation is a first-class path, not an afterthought.
- **Excellent first-run path** — Automatic detection on first run; no mandatory `clido init` before first use.
- **Deterministic automation path** — Stable JSON schemas, exit codes, and env vars for scripts and CI.
- **Safe by default** — Destructive or state-changing actions require permission or explicit flags.
- **Progressive disclosure** — Simple on first run; power users get profiles, memory, and MCP without clutter for beginners.
- **One obvious way** — Common tasks have one canonical command or flag; aliases exist only for compatibility.
- **Human-readable by default, machine-readable when requested** — Text mode is rich and helpful; `--output-format json` and `--json` per command when needed.
- **Consistency over imitation** — Clido's surface is consistent; we do not blindly copy every Claude/Cursor quirk.

### Non-Goals

- Full-screen TUI in V1 or V1.5.
- Undocumented hidden commands or flags.
- Breaking renames without deprecation aliases.
- Provider-specific UX divergence at the top level (e.g. different flag names per provider).

---

## 1. Personas and User Stories

### Personas

1. **Solo Developer** — Uses Clido daily on personal or small-team repos. Wants fast iteration, clear feedback, and safe edits.
2. **Automation / CI Engineer** — Runs Clido as a subprocess or from scripts. Needs stable output, exit codes, and no interactive prompts.
3. **Power User** — Uses profiles, memory, subagents, MCP, and planner on complex repos. Wants control and observability.

### User Stories by Release

#### V1

| As a… | I want to… | So that… |
|-------|------------|----------|
| Solo Developer | run `clido "fix the test"` and get a result | I can complete small tasks without leaving the terminal. |
| Solo Developer | be prompted before Edit/Write/Bash | I do not accidentally overwrite files. |
| Solo Developer | resume a session after Ctrl-C | I do not lose context when I interrupt. |
| Automation Engineer | run `clido -p "task"` with no TTY | I can script Clido in CI. |
| Solo Developer | see which tool is running and the result | I understand what Clido is doing. |

**Acceptance:** Multi-turn tasks complete; permission prompts appear for state-changing tools; `--resume` restores session; non-TTY runs without hanging.

#### V1.5

| As a… | I want to… | So that… |
|-------|------------|----------|
| Automation Engineer | get JSON output with exit status | I can branch on success/failure in scripts. |
| Solo Developer | see cost per session | I can control spend. |
| Solo Developer | run `clido doctor` | I can fix setup issues quickly. |
| Automation Engineer | use `--quiet` | I get only the final answer in text mode. |

**Acceptance:** `--output-format json` has stable schema and exit codes; cost appears in footer; doctor runs and reports config/provider/storage.

#### V2

| As a… | I want to… | So that… |
|-------|------------|----------|
| Power User | switch provider/model via `--profile` or `--model` | I can use different models for different tasks. |
| Power User | run `clido audit` and `clido stats` | I can inspect tool usage and cost. |
| Automation Engineer | use `--input-format stream-json` | I can drive Clido from an SDK. |
| Solo Developer | use shell completion | I can discover commands and flags. |

**Acceptance:** Profiles and overrides work; audit and stats output match spec; completions install and work.

#### V3

| As a… | I want to… | So that… |
|-------|------------|----------|
| Power User | use `clido memory list` and `clido memory prune` | I can manage what Clido remembers. |
| Power User | call SemanticSearch from the model | I get semantic code search. |
| Power User | run MCP tools with clear permission prompts | I can extend Clido safely. |

**Acceptance:** Memory commands work; SemanticSearch is available to the model; MCP tools appear and respect permissions.

#### V4

| As a… | I want to… | So that… |
|-------|------------|----------|
| Power User | run with `--planner` for large refactors | I get a plan before execution. |

**Acceptance:** Planner mode is optional and fallback to reactive loop is seamless.

---

## 2. Per-Release Command Surface Map

### Command and flag matrix

| Command / Flag | V1 | V1.5 | V2 | V3 | V4 |
|----------------|----|------|----|----|-----|
| `clido <prompt>` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido run <prompt>` | — | — | ✓ | ✓ | ✓ |
| `clido --version` / `clido version` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --continue` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --resume <id>` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --print` / `-p` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --tools <list>` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --allowed-tools <list>` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --disallowed-tools <list>` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --permission-mode` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --profile` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --model` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --provider` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --system-prompt` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --system-prompt-file` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --append-system-prompt` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --max-turns` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --max-budget-usd` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --output-format` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --no-color` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --verbose` / `-v` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido --quiet` / `-q` | — | ✓ | ✓ | ✓ | ✓ |
| `clido --input-format` | — | — | ✓ | ✓ | ✓ |
| `clido --mcp-config <file>` | — | ✓ | ✓ | ✓ | ✓ |
| `clido --sandbox` | — | — | ✓ | ✓ | ✓ |
| `clido --planner` | — | — | — | — | ✓ |
| `clido sessions list` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido sessions show <id>` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido sessions fork <id>` | — | ✓ | ✓ | ✓ | ✓ |
| `clido doctor` | — | ✓ | ✓ | ✓ | ✓ |
| `clido audit` | — | — | ✓ | ✓ | ✓ |
| `clido stats` | — | — | ✓ | ✓ | ✓ |
| `clido list-models` | — | ✓ | ✓ | ✓ | ✓ |
| `clido update-pricing` | — | ✓ | ✓ | ✓ | ✓ |
| `clido init` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `clido completions <shell>` | — | — | ✓ | ✓ | ✓ |
| `clido man` | — | — | ✓ | ✓ | ✓ |
| `clido memory list` | — | — | — | ✓ | ✓ |
| `clido memory prune` | — | — | — | ✓ | ✓ |
| `clido memory reset` | — | — | — | ✓ | ✓ |
| `clido fetch-models` | — | — | — | ✓ | ✓ |
| SemanticSearch tool | — | — | — | ✓ | ✓ |
| ExitPlanMode tool | ✓ | ✓ | ✓ | ✓ | ✓ |

### UX promise matrix

| UX Capability | V1 | V1.5 | V2 | V3 | V4 |
|---------------|----|------|----|----|-----|
| Rich streaming text output | ✓ | ✓ | ✓ | ✓ | ✓ |
| Deterministic JSON contract | — | ✓ | ✓ | ✓ | ✓ |
| Provider switching UX | — | — | ✓ | ✓ | ✓ |
| Shell completion | — | — | ✓ | ✓ | ✓ |
| Session browsing UX | ✓ | ✓ | ✓ | ✓ | ✓ |
| Audit and stats commands | — | — | ✓ | ✓ | ✓ |
| Memory commands | — | — | — | ✓ | ✓ |
| Semantic search tool | — | — | — | ✓ | ✓ |
| Planner mode / `--planner` | — | — | — | — | ✓ |

---

## 3. Detailed Command Reference

### Precedence and conflicts

**Configuration precedence:**  
`CLI flag > env var > project config (.clido/config.toml) > user config (~/.config/clido/config.toml) > built-in default`

**Invalid flag combinations (must error at startup):**

- `--resume <id>` and `--continue` together
- `--quiet` and `--verbose` together
- `--system-prompt` and `--system-prompt-file` together (both replace; use `--append-system-prompt` to add)
- `--input-format stream-json` and interactive REPL mode
- `--output-format json` or `stream-json` with flags that imply interactive prompts in a way that cannot be satisfied

### Top-level information architecture

`clido --help` groups commands as follows:

- **Run:** `clido <prompt>`, `run`
- **Sessions:** `sessions list`, `sessions show`, `sessions fork`
- **Health and diagnostics:** `doctor`, `stats`, `audit`
- **Memory:** `memory list`, `memory prune`, `memory reset`
- **Discovery / config:** `list-models`, `update-pricing`, `fetch-models`, `init`, `man`, `completions`, `version`

### Environment variables

| Env var | Equivalent flag | Notes |
|---------|-----------------|-------|
| `CLIDO_MODEL` | `--model` | |
| `CLIDO_PROFILE` | `--profile` | |
| `CLIDO_PROVIDER` | `--provider` | |
| `CLIDO_MAX_TURNS` | `--max-turns` | Integer |
| `CLIDO_MAX_BUDGET_USD` | `--max-budget-usd` | Float |
| `CLIDO_PERMISSION_MODE` | `--permission-mode` | `default`, `accept-all`, `plan` |
| `CLIDO_SYSTEM_PROMPT` | `--system-prompt` | |
| `CLIDO_OUTPUT_FORMAT` | `--output-format` | `text`, `json`, `stream-json` |
| `CLIDO_CONFIG` | (config file path) | Override config location |
| `CLIDO_DATA_DIR` | (data directory) | Override data directory |
| `CLIDO_SESSION_DIR` | (session directory) | Override session directory |
| `CLIDO_LOG` | (log level) | `error`, `warn`, `info`, `debug`, `trace` |
| `NO_COLOR` | `--no-color` | Standard; respected unconditionally |

### Canonical command naming and aliases

**Canonical form:** Noun-first grouped subcommands (e.g. `clido sessions list`, `clido memory prune`).

**Legacy aliases:** `list-sessions` → `sessions list`, `show-session` → `sessions show`. When a legacy name is used, print once to stderr:

```
Warning: 'clido list-sessions' is deprecated. Use 'clido sessions list' instead.
```

Deprecation window: aliases remain for at least one full major release after the canonical name ships.

### Command mode labels

- **interactive-first:** Optimized for human use; degrades gracefully when not a TTY.
- **automation-safe:** Stable flags, schemas, and exit codes for scripts.
- **dual-mode:** Works well both interactively and in automation.

Main agent run: **dual-mode**.  
`sessions list/show/fork`, `doctor`, `stats`, `audit`, `memory list/prune/reset`, `list-models`, `update-pricing`, `init`, `completions`, `man`, `version`: **dual-mode** (all support `--json` or equivalent where applicable).

---

## 4. First-Run and Onboarding Experience

### Auto-detection on first run

Clido does **not** require `clido init` before first use. When no config file exists:

1. Clido starts and detects no `~/.config/clido/config.toml` (or `CLIDO_CONFIG` path).
2. If stdin is a TTY: print a one-time setup header and run an interactive setup flow.
3. Ask: provider (Anthropic / OpenAI / OpenRouter / Local).
4. If cloud: ask for API key (or confirm use of existing env var).
5. If local: ask for base URL (default `http://localhost:11434`).
6. Write minimal config to the config path.
7. Proceed with the user's original task.

### `clido init`

`clido init` exists as an explicit command that runs the same setup flow. It can be re-run to switch providers or reset config. Useful when the user wants to reconfigure without running a task.

### Non-interactive first run

When stdout is not a TTY (or `--print` is set) and no config exists, do **not** prompt. Exit immediately with:

```
Error [Config]: No configuration found. Run 'clido init' to set up Clido.
```

Exit code: `2`.

### Stdin piping and input modes

| Condition | Behavior |
|-----------|----------|
| Positional prompt arg given | Run single-turn with that prompt |
| No arg, stdin is TTY | Enter interactive REPL |
| No arg, stdin is not TTY | Read full stdin as prompt; run single-turn |
| `--print`, no arg, stdin is pipe | Read full stdin as prompt |
| `--print`, no arg, stdin is TTY | Error: no prompt provided |

**Examples:**

- `echo "fix the failing test" | clido` → single-turn with that prompt
- `clido < task.txt` → single-turn with file contents as prompt
- `clido "fix the failing test"` → single-turn with that prompt
- `clido` (TTY) → REPL
- `clido --print` (TTY, no prompt) → `Error [Usage]: No prompt provided. Pass a prompt as an argument or pipe it via stdin.`

### Empty-state output

- **`clido sessions list`** (no sessions): `No sessions yet. Run 'clido <prompt>' to start one.`
- **`clido memory list`** (no memories): `No memories yet. Clido will save insights as you work.`
- **`clido list-models`** (provider unreachable): `Error [Provider]: Could not reach provider. Check your API key and network.`
- **`clido list-models --provider local`** (Ollama not running): `No local models found. Is Ollama running? (ollama serve)`

---

## 5. Text Output Design

### Tool display lifecycle

**Rich TTY mode:**

```
  · Read src/main.rs                    ← pending (spinner)
  ↻ Read src/main.rs                    ← in progress (animated)
  ✓ Read src/main.rs        142 lines   ← completed
  ✗ Edit src/foo.rs         string not found   ← completed with error
```

**ASCII / non-TTY mode:**

```
  [run] Read src/main.rs
  [ok]  Read src/main.rs (142 lines)
  [err] Edit src/foo.rs (string not found)
```

### Edit success: inline diff

From the model's tool result (structured patch), show a short inline diff:

```
  ✓ Edit src/auth/login.rs
    - const active = false
    + const active = true
    (1 line changed)
```

- Maximum 5 lines shown inline; if longer, append `(+N more lines)`.
- Full diff available in `--verbose` mode.

### Parallel tool display

When the model returns N tools in one response and they run concurrently:

**Rich TTY:**

```
  Running 3 tools in parallel:
    ↻ Read src/main.rs
    ↻ Read src/auth/login.rs
    ↻ Read Cargo.toml

  ✓ Read src/main.rs          142 lines
  ✓ Read src/auth/login.rs     89 lines
  ✓ Read Cargo.toml            31 lines
```

**ASCII / non-TTY:** One line per tool as it completes:

```
  [parallel: 3 tools]
  [ok] Read src/main.rs (142 lines)
  [ok] Read src/auth/login.rs (89 lines)
  [ok] Read Cargo.toml (31 lines)
```

If any tool in the batch fails, show it inline after the group, e.g. `✗ Read src/missing.rs   File does not exist`.

### Context compaction UX

Compaction is always visible. Never silent.

**Rich (transient in TTY, e.g. clears after 2s):**

```
  ↻ Compacting context  (12,847 tokens → ~600 token summary)...
  ✓ Context compacted
```

**ASCII (persistent):**

```
  [compact] Compacting context (12,847 tokens)...
  [compact] Done - summary injected
```

In `--output-format stream-json`, emit:

```json
{ "type": "system", "subtype": "compact_start", "context_tokens": 12847 }
{ "type": "system", "subtype": "compact_end", "summary_tokens": 600 }
```

### Other output elements

- **Streaming assistant text:** Raw to stdout, no buffering.
- **Thinking indicator:** `Thinking...` with spinner (rich); `[thinking]` (ASCII).
- **Session footer on completion:** `✓ Done  ·  5 turns  ·  $0.0041  ·  2.3s`
- **ExitPlanMode transition:** `⚡ Switching to agent mode — state-changing tools now available` (rich); `[mode] Switching to agent mode - state-changing tools now available` (ASCII).
- **`--verbose`:** Full tool inputs/outputs; timestamps per event.
- **`--quiet`:** No spinner, no tool lifecycle, no cost footer; only final response and errors.
- **Redraw policy:** Completed tool lines stay visible; only the current in-progress line is transient/animated.
- **Reasoning/thinking tokens:** Hidden by default; in `--verbose` show as `[thinking] <text>`.
- **Long-running Bash:** After 3s show elapsed time, e.g. `↻ Bash cargo build  (14s)`.

### Terminal behavior matrix

| Environment | Color | Unicode | Spinner | Prompting | Layout |
|-------------|-------|---------|---------|-----------|--------|
| Interactive TTY | on | on | on | yes | rich |
| Non-TTY / pipe | off | ASCII | off | no | compact |
| `TERM=dumb` | off | ASCII | off | minimal | plain text |
| `NO_COLOR=1` | off | unchanged | off (optional) | yes (if TTY) | plain text |
| Narrow (<60 cols) | on | on | on | yes | truncated |
| Windows legacy console | limited | ASCII fallback | off | yes | plain text |

**ASCII fallback symbols:** `[ok]`, `[error]`, `[run]`, `[warn]`, `[mode]`, `[compact]`, `[thinking]`, `[parallel]`, `-`, `|`, `+`

**Width rules:**

- Minimum width: 60 columns.
- File paths: truncate with `...` in the middle at 40 chars in compact mode.
- Preview text: truncate at 50 chars with `…`.
- Tables: collapse to two-column layout below 80 columns.

---

## 6. Error Message Standards

### CLI-level errors (config, startup)

Shown on stderr. Format:

```
Error [<Category>]: <message>

  <actionable hint>
```

**Example:**

```
Error [Config]: Missing API key for provider 'anthropic'

  Set ANTHROPIC_API_KEY in your environment or in ~/.config/clido/config.toml
  Run: clido doctor  to check all configuration
```

**Categories:** `Config`, `Provider`, `Tool`, `Session`, `Budget`, `Permission`, `Usage`

### Tool errors (passed back to model)

Exact formats (for model consumption and session replay):

- **Edit failure:** `<tool_use_error>String to replace not found in file.\nString: <old_string></tool_use_error>`
- **Read failure:** `File does not exist. Note: your current working directory is <cwd>.`
- **Bash failure:** `Exit code 1\n<stderr text>`
- **Grep validation:** `<tool_use_error>InputValidationError: Grep failed due to the following issue:\nAn unexpected parameter \`<key>\` was provided</tool_use_error>`

### Partial-success presentation

- **budget_exceeded:** Footer: `⚠ Session stopped — budget limit reached ($0.10). Use --max-budget-usd to raise the limit.`
- **max_turns_reached:** Footer: `⚠ Session stopped — turn limit reached (20 turns). Use --max-turns to raise the limit.`

### Exit codes (unified)

**Main agent and most commands:**

| Code | Meaning |
|------|---------|
| `0` | Success / completed |
| `1` | Runtime error (provider, tool, session) |
| `2` | Usage / config error (bad flags, missing key) |
| `3` | Soft limit reached (budget or turns) |

**`clido doctor` only:**

| Code | Meaning |
|------|---------|
| `0` | All checks passed |
| `1` | One or more mandatory checks failed |
| `2` | Mandatory passed; one or more optional warnings |

### Secret redaction

Any error or log line that would display an API key or token is redacted, e.g. `ANTHROPIC_API_KEY=sk-ant-***...***`.

### Retry hints

Show a retry hint only when retry is applicable (e.g. 429 → "Retry in 30s or reduce --max-turns").

---

## 7. Interactive Permission Prompt UX

### Rich TTY mode

```
┌─ Permission required ──────────────────────────────────────┐
│ Tool:   Edit                                               │
│ File:   src/auth/login.rs                                  │
│ Change: replace 47 chars on line 23                        │
└────────────────────────────────────────────────────────────┘
Allow? [y] yes  [n] no  [a] always  [d] disallow  [?] help
```

### ASCII fallback

```
  [permission] Edit src/auth/login.rs (replace 47 chars on line 23)
  Allow? [y]es / [n]o / [a]lways / [d]isallow / [?]help:
```

### Modes

- **Default** (`--permission-mode default`): Prompt for state-changing tools.
- **AcceptAll** (`--permission-mode accept-all`): No prompt; startup banner: `⚡ Running in accept-all mode — all tool calls are auto-approved`
- **PlanOnly** (`--permission-mode plan`): No prompt; state-changing tools denied with inline: `[plan mode] Edit blocked — ExitPlanMode to allow`
- **Serialized gate:** Only one permission prompt visible at a time (shared mutex).

There is no `--yolo` alias; use `--permission-mode accept-all`.

### EOF / timeout

- EOF while waiting for answer → treat as **deny**.
- Ctrl-C while prompting → cancel current turn; print `Turn cancelled.`
- No configurable timeout in V1; may be added later.

---

## 8. REPL (Interactive Mode) UX

**Entry:** `clido` with no arguments and stdin is a TTY.

**Prompt:** `clido> `  
Optional: cost in right gutter when wide enough, e.g. `[$0.0042]`.

**Input:**

- Single line: type and Enter.
- Multi-line: end line with `\` to continue, or paste multiple lines at once.
- History: arrow keys and Ctrl-R search (e.g. via rustyline).

**Slash commands (REPL-local, not sent to the agent):**

- `/help` — REPL command list (not full CLI help).
- `/cost` — Current session cost and turn count.
- `/sessions` — Quick session list.
- `/resume <id>` — Resume a session in this REPL.
- `/mode plan` — Switch to plan mode for remaining turns.
- `/mode agent` — Switch to agent mode (same as ExitPlanMode).
- `/exit` or `/quit` — Clean exit.
- `//` — Send a literal prompt starting with `/` (e.g. `//help me with...`).

**Exit:**

- First Ctrl-C: cancel in-progress turn; print `Turn cancelled. Ctrl-C again to exit.`
- Second Ctrl-C or `/exit`: clean exit; session saved.

**Session:** One continuous session per REPL invocation; all turns share the same session ID and file.

**ASCII:** Prompt remains `clido> `; no gutter cost indicator.

---

## 9. Session Management UX

### `clido sessions list`

```
  ID              Date         Turns   Cost      Preview
  ──────────────  ───────────  ──────  ────────  ────────────────────────────
  a3f2...9b1c     2026-03-15       8   $0.0341   "audit the auth module for …"
  e7d1...2a4f     2026-03-14       3   $0.0091   "fix the failing test in …"
```

- Sort: newest first.
- ID: UUID v4, display as first 8 chars + `...` + last 4 chars (e.g. `a3f2b1c9...9b1c`).
- Preview: first user message, truncated at 50 chars with `…`.
- Cost: `$0.XXXX` (4 decimal places).
- `--json` supported.

### `clido sessions show <id>`

Full transcript replay. `--json` supported.

### `clido sessions fork <id>`

Fork from that session; new session ID. `--json` supported.

### `--continue`

Resumes the **newest** session for the **current project path** (cwd-matched). Not global.

---

## 10. CI / Non-Interactive Mode Contract

### Auto non-interactive

When stdout is not a TTY, Clido behaves as if `--print` were set. `AskUser` permission prompts auto-deny with a warning to stderr.

### `--output-format json`

Single JSON object on stdout at completion. Warnings go to stderr.

Example shape:

```json
{
  "schema_version": 1,
  "type": "result",
  "exit_status": "completed",
  "result": "...",
  "session_id": "a3f2b1c9-...",
  "num_turns": 5,
  "duration_ms": 12345,
  "total_cost_usd": 0.0045,
  "is_error": false,
  "usage": {
    "input_tokens": 1234,
    "cache_read_input_tokens": 890,
    "output_tokens": 567
  }
}
```

`exit_status`: `completed`, `max_turns_reached`, `budget_exceeded`, `error`.

### `--output-format stream-json`

Newline-delimited JSON events on stdout. Example events:

- `{ "type": "text", "text": "..." }`
- `{ "type": "tool_use", "name": "Read", "input": {...} }`
- `{ "type": "tool_result", "name": "Read", "is_error": false, "content": "..." }`
- `{ "type": "system", "subtype": "compact_start", "context_tokens": 12847 }`
- `{ "type": "system", "subtype": "compact_end", "summary_tokens": 600 }`
- `{ "type": "result", "exit_status": "completed", "schema_version": 1, ... }`

Progress events (tool pending/in_progress) are not included by default; use `--verbose` to include them.

---

## 11. `clido doctor` Output Design

Version-aware checks (see development-plan Phase 8.4.1). Example:

```
$ clido doctor

  Clido v0.1.0  (V1 checks)

  ✓  rustc 1.78.0
  ✓  ANTHROPIC_API_KEY set
  ✓  Provider reachable (anthropic, 243ms)
  ✓  Session directory writable (~/.local/share/clido/sessions)
  ✓  pricing.toml present
  ⚠  pricing.toml is 94 days old — run 'clido update-pricing' to refresh

  1 warning. All required checks passed.
  Exit code: 2
```

- `✓` = passed; `✗` = failed (mandatory); `⚠` = warning (optional).
- Offline mode: skipped checks show `[skipped — offline mode]`.
- Every `✗` line includes a one-line remediation hint.
- `clido doctor --json`: machine-readable array of check results.
- Check ordering: stable within each version group (e.g. alphabetical).
- Exit codes: `0` = all pass, `1` = mandatory fail, `2` = warnings only.

---

## 12. `clido audit` UX

**Availability:** V2 (requires audit log).

```
$ clido audit
$ clido audit --tail
$ clido audit --session <id>
$ clido audit --tool Edit
$ clido audit --since 2026-03-01
$ clido audit --json
```

Default (newest first, tabular):

```
  Timestamp            Session   Tool    Input Summary         Result
  ───────────────────  ────────  ──────  ────────────────────  ───────
  2026-03-16 14:23:01  a3f2...   Edit    src/main.rs           [ok]
  2026-03-16 14:22:58  a3f2...   Read    src/main.rs           [ok]
  2026-03-16 14:22:50  a3f2...   Bash    cargo test            [exit=1]
```

`--json`: stable schema with `schema_version`.

---

## 13. `clido stats` UX

**Availability:** V2 (requires telemetry).

```
$ clido stats
$ clido stats --session <id>
$ clido stats --json
```

Default:

```
  Clido usage summary

  Tool calls                         Avg latency
  ──────────────────────────────     ───────────
  Read        1,173   (55.0%)        45ms
  Bash        1,043   (49.0%)        2.1s
  Edit          646   (30.3%)        120ms

  Sessions: 80   Total cost: $4.23   Avg: $0.053/session
  Provider retries: 12 (1.2%)
```

---

## 14. Shell Completion and Discovery

- **Bash:** `clido completions bash` → write to e.g. `~/.bash_completion.d/clido`
- **Zsh:** `clido completions zsh` → write to e.g. `~/.zsh/completions/_clido`
- **Fish:** `clido completions fish` → write to e.g. `~/.config/fish/completions/clido.fish`
- **Help:** `clido --help` and `clido help <cmd>` / `clido <cmd> --help` (both supported).
- **Man:** `clido man` prints the man page to stdout.

Man page sections: NAME, SYNOPSIS, DESCRIPTION, OPTIONS, SUBCOMMANDS, ENVIRONMENT, FILES, EXIT STATUS, EXAMPLES, SEE ALSO.

---

## 15. Accessibility and Portability

- Status is conveyed by **symbol and text**; never by color or glyph alone.
- ASCII mode provides full information without Unicode.
- All examples use copy-paste-safe characters (no smart quotes or invisible chars).
- In docs, Windows path examples may use forward slashes (Clido normalizes).
- Color is not the only distinction (e.g. green/red); symbols carry meaning.

---

## 16. Deprecation and Compatibility Policy

- **Aliases:** Renamed commands/flags keep the old name as a hidden alias for at least one full major release.
- **Deprecation notice:** `Warning: '<old>' is deprecated. Use '<new>' instead.` — printed to stderr once per session.
- **JSON schema:** Breaking changes increment `schema_version`; additive-only changes are non-breaking.
- **Required per CLI change:** Spec updated, `--help` text updated, test added, changelog entry, release doc updated.
- **Breaking changes:** Must appear in CHANGELOG under a "Breaking Changes" heading.

---

## 17. Validation Checklist (Spec Completeness)

Before considering the spec complete for a release:

- [ ] Every command has exact syntax.
- [ ] Every global flag has type, default, env var (if any), and conflict rules.
- [ ] Every release has a complete CLI surface entry in the surface map.
- [ ] Every interactive surface has both rich and ASCII behavior defined.
- [ ] Every machine mode has exit code and JSON schema (where applicable).
- [ ] Every empty state has defined copy.
- [ ] Every error category has a message template and hint.
- [ ] Roadmap and release docs reference this spec.
- [ ] New UX behaviors have corresponding tests in the testing strategy.

---

*This specification is the single source of truth for Clido's CLI. The development plan and release documents reference it and must stay consistent with it.*
