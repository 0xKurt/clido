# Architecture

Developer reference for the internal structure of **cli;do**.

## Crate Overview

All crates live under `crates/`. The dependency graph flows downward—foundation crates have no local dependencies, application crates depend on everything above.

| Crate | Purpose |
|---|---|
| **clido-core** | Shared types, errors, config schema, and `ProviderType` enum. Zero local deps—everything else depends on this. |
| **clido-providers** | `ModelProvider` trait and implementations (Anthropic native SDK + OpenAI-compatible generic). Handles API serialization, streaming, model aliases, and provider registry. |
| **clido-tools** | `Tool` trait, tool registry, and built-in tool implementations: Bash, Read, Write, Edit, Glob, Grep, Git, Web Search/Fetch, Test Runner, SemanticSearch, and MCP bridge. |
| **clido-agent** | Agent loop (`AgentLoop`) that orchestrates prompt→provider→tool→response turns. Defines the `EventEmitter` trait for push-based UI updates. |
| **clido-context** | Context assembly and token-budget management. Truncates or compacts conversation history to fit model context windows. |
| **clido-storage** | Session persistence (JSONL), audit logging, and XDG data-directory management. |
| **clido-memory** | Long-term memory store backed by SQLite with FTS5 keyword search. Memories are auto-injected into system prompts. |
| **clido-index** | Repository file and symbol indexer (SQLite). Powers the `SemanticSearch` tool for code navigation. |
| **clido-checkpoint** | Content-addressed file snapshots stored under `.clido/checkpoints/<session>/<id>/`. Used by rollback. |
| **clido-planner** | Task planner that decomposes prompts into a DAG and executes subtasks in dependency order with optional parallelism. Activated by `--planner`. |
| **clido-workflows** | Declarative YAML workflow engine: load, validate, template, and execute multi-step workflows with a step-runner abstraction. |
| **clido-cli** | Main binary crate. CLI argument parsing, TUI rendering, REPL mode, and all user-facing commands. Depends on every other crate. |

### Dependency layers

```
┌─────────────────────────────────────┐
│            clido-cli                │  ← application (binary)
├─────────────────────────────────────┤
│            clido-agent              │  ← orchestration
├──────────┬──────────┬───────────────┤
│ clido-   │ clido-   │ clido-        │
│ context  │ tools    │ workflows     │  ← domain logic
├──────────┼──────────┼───────────────┤
│ clido-   │ clido-   │ clido-  clido-│
│ providers│ storage  │ memory  index │  ← infrastructure
├──────────┴──────────┼───────────────┤
│ clido-checkpoint    │ clido-planner │  ← standalone utilities
├─────────────────────┴───────────────┤
│            clido-core               │  ← foundation (types, config)
└─────────────────────────────────────┘
```

## Module Map — clido-cli

The `clido-cli` crate (`crates/clido-cli/src/`) contains the main binary and all user-facing logic:

| Module | Description |
|---|---|
| `main.rs` | Entry point — dispatches to TUI, REPL, or one-shot run based on args and TTY detection. |
| `cli.rs` | CLI argument parsing (clap). Defines all flags, options, and subcommands. |
| `tui.rs` | Full-screen interactive TUI built on ratatui. Contains the `App` struct, render loop, input handling, and all TUI state. This is the largest module (~11 000 lines). |
| `text_input.rs` | Reusable single-line text input widget with cursor, word operations, history, masked input (for API keys), and placeholder text. |
| `list_picker.rs` | Generic filterable list widget with selection, scrolling, wrapping navigation, and the `PickerItem` trait. Used by model, session, profile, and role pickers. |
| `overlay.rs` | `OverlayStack` system for modal UI layers. Provides `ErrorOverlay`, `ReadOnlyOverlay`, `ChoiceOverlay`, and routes input to the topmost overlay. |
| `command_registry.rs` | Slash command definitions. Each command has a name, description, category, argument spec, and idle-required flag. 53 commands across 7 categories. |
| `setup.rs` | First-run wizard and `clido init` — interactive profile creation with provider/model/key prompts. |
| `config.rs` | Config resolution — merges global config, project-local `.clido/config.toml`, env vars, and CLI flags. |
| `profiles.rs` | Profile CRUD operations (list, create, switch, edit, delete). |
| `provider.rs` | Provider instantiation — builds a `ModelProvider` from resolved config. |
| `agent_setup.rs` | Agent initialization — wires up provider, tools, permissions, and event emitter into an `AgentLoop`. |
| `run.rs` | One-shot `clido run` / `clido "prompt"` execution path. |
| `repl.rs` | Non-TUI REPL mode for headless / piped usage. |
| `models.rs` | Model listing and display logic for `clido list-models`. |
| `sessions.rs` | Session list/show/fork subcommands. |
| `doctor.rs` | `clido doctor` — checks environment, API keys, config, and tool health. |
| `commit.rs` | Git commit helpers for `/ship`, `/save`, `/pr`. |
| `git_context.rs` | Git context utilities (branch, diff, status) injected into agent context. |
| `image_input.rs` | Image attachment handling for `/image` command and multimodal input. |
| `prompt_enhance.rs` | Prompt enhancement pipeline — rewrites user prompts before sending. |
| `spawn_tools.rs` | Tool spawning and parallel execution management. |
| `notify.rs` | Desktop notification support (macOS/Linux). |
| `stats.rs` | `clido stats` — session statistics and cost reporting. |
| `ui.rs` | Shared UI rendering utilities (colors, borders, layout helpers). |
| `errors.rs` | CLI-specific error types and display formatting. |
| `audit_cmd.rs` | `clido audit` subcommand handler. |
| `checkpoint_cmd.rs` | `clido checkpoint` / `clido rollback` subcommand handlers. |
| `index_cmd.rs` | `clido index` subcommand handler. |
| `memory_cmd.rs` | `clido memory` subcommand handler. |
| `plan_cmd.rs` | `clido plan` subcommand handler. |
| `pricing_cmd.rs` | `clido update-pricing` / `clido fetch-models` handlers. |
| `workflow.rs` | `clido workflow` subcommand handler. |

## Data Flow

How a user prompt flows from input to displayed response:

```
┌──────────────────────────────────────────────────────────────────┐
│  TUI (tui.rs)                                                    │
│                                                                  │
│  1. User types in TextInput and presses Enter                    │
│  2. App::handle_key() extracts text, sends via prompt_tx channel │
└──────────────────────┬───────────────────────────────────────────┘
                       │  prompt_tx (tokio mpsc)
                       ▼
┌──────────────────────────────────────────────────────────────────┐
│  Agent Task (runs in background tokio task)                      │
│                                                                  │
│  3. Receives prompt via prompt_rx                                │
│  4. AgentLoop::run_next_turn(user_input)                         │
│     a. Appends UserMessage to conversation history               │
│     b. Injects memories + context (clido-context)                │
│     c. Calls provider.complete(history, tool_schemas)            │
└──────────────────────┬───────────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────────┐
│  Provider (clido-providers)                                      │
│                                                                  │
│  5. Serializes messages to provider wire format                  │
│  6. HTTP request to LLM API (Anthropic / OpenAI-compatible)     │
│  7. Deserializes response → ModelResponse                        │
└──────────────────────┬───────────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────────┐
│  Agent Loop (continued)                                          │
│                                                                  │
│  8. If stop_reason == EndTurn:                                   │
│       → Emit text via EventEmitter::on_assistant_text()          │
│       → Write turn to SessionWriter (clido-storage)              │
│       → Done                                                     │
│                                                                  │
│  9. If stop_reason == ToolUse:                                   │
│       a. Emit on_tool_start() for each tool call                 │
│       b. Check permissions (ask user if needed)                  │
│       c. Execute tools (parallel for read-only tools)            │
│       d. Emit on_tool_done() with results                        │
│       e. Append tool results to history                          │
│       f. Write to SessionWriter                                  │
│       g. Loop back to step 4c (next provider call)               │
└──────────────────────┬───────────────────────────────────────────┘
                       │  EventEmitter callbacks → AgentEvent channel
                       ▼
┌──────────────────────────────────────────────────────────────────┐
│  TUI Render Loop                                                 │
│                                                                  │
│  10. Polls AgentEvent channel every render tick (~80ms)           │
│  11. Updates App::messages with new content                      │
│  12. Repaints via ratatui                                        │
└──────────────────────────────────────────────────────────────────┘
```

### Key channels in the TUI App

| Channel | Direction | Purpose |
|---|---|---|
| `prompt_tx` | TUI → agent | User prompts |
| `resume_tx` | TUI → agent | Resume session requests |
| `model_switch_tx` | TUI → agent | Model changes mid-session |
| `workdir_tx` | TUI → agent | Working directory updates |
| `compact_now_tx` | TUI → agent | Trigger immediate context compaction |
| `fetch_tx` / event channel | Agent → TUI | `AgentEvent` variants (ToolStart, ToolDone, Thinking, BudgetWarning) |

### Permission flow

Before executing state-changing tools (Bash, Write, Edit), the agent asks the user:

1. Agent calls `ask_user.ask(PermRequest)` with tool name, args, and affected paths
2. TUI shows a permission popup overlay
3. User selects: **Once** / **Session** / **Workdir** / **Deny** / **Deny + feedback**
4. Response flows back to the agent loop
5. Agent either executes the tool or skips it (with optional deny-reason injected as context)

## Key Architectural Patterns

### EventEmitter trait

Defined in `clido-agent/src/agent_loop.rs`. This is the push-based interface for agent→TUI communication:

```rust
pub trait EventEmitter: Send + Sync {
    async fn on_tool_start(&self, tool_use_id: &str, name: &str, input: &Value);
    async fn on_tool_done(&self, tool_use_id: &str, name: &str, is_error: bool, diff: Option<String>);
    async fn on_assistant_text(&self, text: &str) {}
    async fn on_budget_warning(&self, pct: u8, spent_usd: f64, limit_usd: f64) {}
}
```

The TUI implements this as `TuiEmitter`, which converts each callback into an `AgentEvent` enum variant and sends it through a tokio channel to the render loop.

### OverlayStack

Defined in `clido-cli/src/overlay.rs`. Replaces a proliferation of `Option<XxxState>` fields with a unified stack:

- **Stack-based dispatch**: the topmost overlay receives all key input; rendering paints bottom-to-top.
- **`OverlayAction` enum**: overlays return `Consumed`, `Dismiss`, `Push(new)`, `Replace(new)`, `Action(app_action)`, or `NotHandled`.
- **Built-in overlays**: `ErrorOverlay`, `ReadOnlyOverlay` (scrollable text), `ChoiceOverlay` (yes/no/cancel).
- Pickers (model, session, profile, role) are handled in `tui.rs` using `ListPicker` but will migrate to the overlay stack over time.

### TextInput / ListPicker primitives

**TextInput** (`text_input.rs`): Single-line editable buffer with cursor positioning, word-level operations (Ctrl+W, Alt+Left/Right), input history (up/down), masked mode for secrets, and placeholder text. Used for the main chat input bar and all text fields in overlays.

**ListPicker** (`list_picker.rs`): Generic `ListPicker<T: PickerItem>` with case-insensitive substring filtering, wrapping selection, scroll management, and page navigation. Does not handle rendering—overlay code reads its state and draws the list. Used for model picker, session picker, profile picker, and role picker.

### Profile / config system

Config is defined in `clido-core/src/config.rs`:

- **`AgentConfig`** — top-level agent behavior: model, max_turns, budget, permission_mode, context limits.
- **`AgentSlotConfig`** — one agent slot (main/worker/reviewer): provider, model, api_key, base_url.
- **`AgentsConfig`** — tiered agent setup with required `main` slot plus optional `worker` and `reviewer`.
- **`RolesConfig`** — named role→model mappings (fast, reasoning, critic, planner).
- **`PermissionRule`** — glob-based file permission rules (allow/ask/deny per path pattern).

Config resolution (`clido-cli/src/config.rs`) merges sources in priority order:
1. CLI flags (highest)
2. Environment variables (`CLIDO_*`)
3. Project-local `.clido/config.toml`
4. Global `~/.config/clido/config.toml`

Profiles are named sections within `config.toml` under `[profiles.<name>]`.

## Style Guide Notes

### Commit trailer

All commits must include:

```
Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

### Pre-commit hooks

The repo uses `.githooks/pre-commit` (activate with `git config core.hooksPath .githooks`). The hook runs:

1. `cargo fmt --check` — formatting must pass
2. `cargo clippy --workspace -- -D warnings` — all clippy warnings are errors

### Rust toolchain

Pinned to **Rust 1.94** via `rust-toolchain.toml`. Use rustup (not Homebrew) to manage the toolchain.

### Test organization

| Location | Type | Description |
|---|---|---|
| `crates/*/src/*.rs` (`#[cfg(test)]`) | Unit tests | Per-module tests embedded in source files |
| `crates/*/tests/*.rs` | Integration tests | Cross-module tests per crate |
| `tests/` | End-to-end | Workspace-level integration tests |

Run tests with:

```sh
cargo test --workspace              # all tests
cargo nextest run --workspace       # parallel execution (faster)
```

Live provider tests (hitting real APIs) are gated behind `CLIDO_LIVE_PROVIDER_TESTS=1` and disabled by default.

### Build commands

```sh
cargo build --workspace                          # debug build
cargo build --release                            # release build
cargo clippy --workspace -- -D warnings          # lint
cargo fmt --check                                # format check
cargo test --workspace                           # test
cargo bench -p clido-cli                         # benchmarks
```
