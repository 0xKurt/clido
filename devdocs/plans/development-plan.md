# Clido — Rust CLI Coding Agent: Development Roadmap

**Project:** `clido` — a local-first, multi-provider CLI coding agent in Rust
**Based on:** Reverse-engineering of Claude CLI and Cursor agent (see `devdocs/REPORT.md`, `devdocs/ARTIFACTS.md`)
**Target:** Production-ready system that reproduces and improves on modern CLI coding agents

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Rust Workspace Structure](#rust-workspace-structure)
3. [Phase 1 — Foundation Setup](#phase-1--foundation-setup)
4. [Phase 2 — Proof of Concept](#phase-2--proof-of-concept)
5. [Phase 3 — Minimal Viable Agent](#phase-3--minimal-viable-agent)
6. [Phase 4 — Feature Expansion](#phase-4--feature-expansion)
7. [Phase 5 — Reliability Improvements](#phase-5--reliability-improvements)
8. [Phase 6 — Performance Optimization](#phase-6--performance-optimization)
9. [Phase 7 — Security and Sandboxing](#phase-7--security-and-sandboxing)
10. [Phase 8 — Developer Experience](#phase-8--developer-experience)
11. [Phase 9 — Production Readiness](#phase-9--production-readiness)
12. [Dependency Map](#dependency-map)
13. [Recommended Crates Reference](#recommended-crates-reference)

---

## Architecture Overview

```
clido (workspace)
│
├── crates/
│   ├── clido-cli/        # CLI entry point (clap, streaming output, plan display)
│   ├── clido-agent/      # Agent loop, turn management, session state, subagents
│   ├── clido-tools/      # Tool trait + all tool implementations
│   ├── clido-context/    # Context assembly, token budgeting, compaction
│   ├── clido-providers/  # Model provider abstraction + implementations
│   ├── clido-storage/    # Session persistence, project config
│   ├── clido-memory/     # Short-term + long-term memory (sqlite/sled)
│   ├── clido-planner/    # Task graph, planner trait, DAG executor (optional advanced)
│   ├── clido-index/      # Repository indexing: tree-sitter, symbol index (optional)
│   └── clido-core/       # Shared types, errors, config structs
```

**Execution flow (from trace evidence):**

```
User input
  → Context engine assembles: system_prompt + tool_guidance + history + tool_results
  → Provider sends request to model
  → Model returns: text block and/or tool_use blocks
  → Tool executor runs tools (parallel for read-only; sequential for state-changing;
    bounded by semaphore)
  → Results appended to history as tool_result user blocks
  → Repeat until no tool_use in response or max_turns reached
  → Emit result (duration_ms, num_turns, total_cost_usd, usage)
```

**Optional planner flow (Phase 4.8+):**

```
User input
  → Planner model call → structured TaskGraph (JSON)
  → Deterministic DAG executor resolves dependencies
  → Tool execution / subagents per task node
  → Reflection step: model reviews results, updates graph if needed
  → Final output
```

---

## Rust Workspace Structure

```
clido/
├── Cargo.toml                  # workspace manifest
├── Cargo.lock
├── .cargo/
│   └── config.toml             # profile optimizations, target settings
├── crates/
│   ├── clido-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs        # thiserror error types
│   │       ├── types.rs        # Message, ContentBlock, ToolUse, ToolResult, etc.
│   │       └── config.rs       # AgentConfig, ProviderConfig
│   ├── clido-tools/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs     # ToolRegistry
│   │       ├── cache.rs        # LRU file read cache
│   │       ├── schema.rs       # JSON Schema generation
│   │       ├── read.rs
│   │       ├── write.rs
│   │       ├── edit.rs
│   │       ├── glob.rs
│   │       ├── grep.rs
│   │       ├── bash.rs
│   │       └── mcp.rs          # MCP client tool wrapper
│   ├── clido-providers/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs     # ModelProvider trait
│   │       ├── anthropic.rs
│   │       ├── openai.rs
│   │       ├── openrouter.rs
│   │       ├── alibaba.rs
│   │       └── local.rs
│   ├── clido-context/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── builder.rs      # ContextBuilder
│   │       ├── compaction.rs   # token counting + compaction
│   │       ├── guidance.rs     # tool usage guidance prompt injection
│   │       └── project.rs      # CLIDO.md / CLAUDE.md loader
│   ├── clido-storage/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── session.rs      # JSONL session read/write
│   │       └── paths.rs        # XDG / platform paths
│   ├── clido-memory/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── short_term.rs   # in-session working memory
│   │       ├── long_term.rs    # cross-session persistent memory (sqlite)
│   │       └── retrieval.rs    # memory lookup by relevance
│   ├── clido-planner/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── planner.rs      # Planner trait
│   │       ├── task_graph.rs   # Task, TaskGraph, DAG resolution
│   │       └── executor.rs     # TaskExecutor, dependency-ordered execution
│   ├── clido-index/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── file_index.rs   # file path + metadata index
│   │       ├── symbol_index.rs # tree-sitter symbol extraction
│   │       └── search.rs       # tantivy full-text search
│   ├── clido-agent/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loop.rs         # AgentLoop, turn execution
│   │       ├── executor.rs     # tool dispatch, parallelism, semaphore
│   │       ├── subagent.rs     # SubAgent, SubAgentManager
│   │       ├── permissions.rs  # allow/deny, plan mode
│   │       └── events.rs       # AgentEvent stream, hooks
│   └── clido-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── run.rs
│           │   ├── resume.rs
│           │   ├── list_sessions.rs
│           │   └── doctor.rs
│           └── output/
│               ├── streaming.rs
│               ├── plan_display.rs   # live plan visualization
│               └── json.rs
├── tests/
│   ├── integration/
│   └── fixtures/
└── devdocs/
```

---

## Phase 1 — Foundation Setup

**Goal:** Working Rust workspace with shared types and CI. No agent logic yet.
**Exit criteria:** `cargo build --workspace` succeeds; all crates compile; basic CI runs.

---

### Milestone 1.1 — Workspace Initialization

#### 1.1.1 Initialize the Rust workspace

1. Run `cargo init --name clido` in the repo root to get a workspace skeleton (or create manually).
2. Replace the root `Cargo.toml` with a workspace manifest:

   ```toml
   [workspace]
   members = [
     "crates/clido-core",
     "crates/clido-tools",
     "crates/clido-providers",
     "crates/clido-context",
     "crates/clido-storage",
     "crates/clido-memory",
     "crates/clido-planner",
     "crates/clido-index",
     "crates/clido-agent",
     "crates/clido-cli",
   ]
   resolver = "2"
   ```

3. Create `.cargo/config.toml` with the content specified in [devdocs/guides/ci-and-release.md](../guides/ci-and-release.md) (§4): `[profile.dev] opt-level = 1` (faster incremental builds), `[profile.release] lto = "thin"`, `codegen-units = 1`.
4. Add a top-level `rust-toolchain.toml` pinning a stable channel (e.g. `channel = "1.78"`).
5. Verify: `cargo metadata --no-deps` outputs all workspace members.

#### 1.1.2 Create all crate skeletons

1. For each crate in `crates/`:
   - `cargo new --lib crates/clido-core`
   - `cargo new --lib crates/clido-tools`
   - `cargo new --lib crates/clido-providers`
   - `cargo new --lib crates/clido-context`
   - `cargo new --lib crates/clido-storage`
   - `cargo new --lib crates/clido-memory`
   - `cargo new --lib crates/clido-planner`
   - `cargo new --lib crates/clido-index`
   - `cargo new --lib crates/clido-agent`
   - `cargo new --bin crates/clido-cli`
2. Each `lib.rs` starts with `// placeholder` only.
3. Verify: `cargo build --workspace` compiles without errors.

#### 1.1.3 Add workspace-level dependencies

1. In root `Cargo.toml` add `[workspace.dependencies]` block with pinned versions:

   ```toml
   [workspace.dependencies]
   tokio = { version = "1", features = ["full"] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   anyhow = "1"
   thiserror = "1"
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter"] }
   clap = { version = "4", features = ["derive"] }
   reqwest = { version = "0.12", features = ["json", "stream"] }
   tokio-stream = "0.1"
   futures = "0.3"
   uuid = { version = "1", features = ["v4"] }
   chrono = { version = "0.4", features = ["serde"] }
   directories = "5"
   glob = "0.3"
   ignore = "0.4"
   regex = "1"
   grep = "0.3"
   async-trait = "0.1"
   toml = "0.8"
   ```

2. Reference these from child crates with `{ workspace = true }` — no version duplication.
3. Run `cargo check --workspace` to confirm resolution.

---

### Milestone 1.2 — Core Types (`clido-core`)

**Dependency:** 1.1 complete.

#### 1.2.1 Define the message/content type hierarchy

1. Open `crates/clido-core/src/types.rs`.
2. Define `Role` enum: `User`, `Assistant`, `System`.
3. Define `ContentBlock` enum (mirrors Anthropic API observed in traces):

   ```rust
   pub enum ContentBlock {
       Text { text: String },
       ToolUse { id: String, name: String, input: serde_json::Value },
       ToolResult { tool_use_id: String, content: String, is_error: bool },
       Thinking { thinking: String },  // for extended thinking support
   }
   ```

4. Derive `Serialize`, `Deserialize`, `Debug`, `Clone` on all types.
5. Define `Message`:

   ```rust
   pub struct Message {
       pub role: Role,
       pub content: Vec<ContentBlock>,
   }
   ```

6. Define `Usage`:

   ```rust
   pub struct Usage {
       pub input_tokens: u64,
       pub output_tokens: u64,
       pub cache_creation_input_tokens: Option<u64>,
       pub cache_read_input_tokens: Option<u64>,
   }
   ```

7. Define `ModelResponse`:

   ```rust
   pub struct ModelResponse {
       pub id: String,
       pub model: String,
       pub content: Vec<ContentBlock>,
       pub stop_reason: StopReason,
       pub usage: Usage,
   }
   ```

8. Define `StopReason` enum: `EndTurn`, `ToolUse`, `MaxTokens`, `StopSequence`.
9. Write unit tests in `types.rs` verifying JSON round-trip for each type.
10. Run `cargo test -p clido-core`.

#### 1.2.2 Define error types

1. Open `crates/clido-core/src/error.rs`.
2. Use `thiserror`:

   ```rust
   #[derive(thiserror::Error, Debug)]
   pub enum ClidoError {
       #[error("provider error: {0}")]
       Provider(String),
       #[error("tool error: {tool_name}: {message}")]
       Tool { tool_name: String, message: String },
       #[error("context limit exceeded: {tokens} tokens")]
       ContextLimit { tokens: u64 },
       #[error("session not found: {session_id}")]
       SessionNotFound { session_id: String },
       #[error("permission denied: {tool_name}")]
       PermissionDenied { tool_name: String },
       #[error("planner error: {0}")]
       Planner(String),
       #[error("io error: {0}")]
       Io(#[from] std::io::Error),
       #[error("json error: {0}")]
       Json(#[from] serde_json::Error),
       #[error(transparent)]
       Other(#[from] anyhow::Error),
   }
   pub type Result<T> = std::result::Result<T, ClidoError>;
   ```

3. Add `clido-core` to `clido-agent`, `clido-tools`, `clido-providers`, `clido-planner`, `clido-memory`, `clido-index` as a dependency.
4. Run `cargo check --workspace`.

#### 1.2.3 Define agent configuration types

1. In `crates/clido-core/src/config.rs`:
   - Define `AgentConfig`: `max_turns: u32`, `max_budget_usd: Option<f64>`, `model: String`, `system_prompt: Option<String>`, `permission_mode: PermissionMode`, `use_planner: bool`, `use_index: bool`.
   - Define `PermissionMode` enum: `Default`, `AcceptAll`, `PlanOnly`.
   - Define `ProviderConfig`: `provider_type: ProviderType`, `api_key: Option<String>`, `base_url: Option<String>`, `model: String`.
   - Define `ProviderType` enum: `Anthropic`, `OpenAI`, `OpenRouter`, `AlibabaCloud`, `Local`.
2. Derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `Default` where appropriate.
3. Write a unit test loading a `AgentConfig` from a JSON string.

---

### Milestone 1.3 — Tracing and Logging Setup

**Dependency:** 1.2 complete.

#### 1.3.1 Initialize tracing in the CLI

1. In `clido-cli/src/main.rs`:

   ```rust
   tracing_subscriber::fmt()
       .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
       .init();
   ```

2. Add `CLIDO_LOG=debug` guidance in README (canonical env: `CLIDO_LOG`, not `RUST_LOG`).
3. Add `tracing::info!`, `tracing::debug!` stubs in `clido-agent` crate.
4. Test: run `CLIDO_LOG=debug cargo run -p clido-cli` and confirm tracing output.

---

### Milestone 1.4 — CI Setup

**Dependency:** 1.1–1.3 complete.

#### 1.4.1 Add GitHub Actions workflow

1. Create `.github/workflows/ci.yml`:
   - `cargo fmt --check`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo test --workspace`
   - `cargo build --workspace --release`
2. Add `.rustfmt.toml` with project formatting preferences.
3. Add `.clippy.toml` or `#![allow(...)]` only for intentional exceptions.
4. Verify CI passes on a clean push.

---

## Phase 2 — Proof of Concept

**Goal:** Single hardcoded model call with a single tool, no persistence. Demonstrates the core loop works end-to-end.
**Exit criteria:** Running `cargo run -p clido-cli -- -p "list files in current directory"` makes one model API call, receives a `tool_use` for `bash`, executes it, feeds result back, and prints final text response.

---

### Milestone 2.1 — Minimal Provider: Anthropic

**Dependency:** Phase 1 complete.

#### 2.1.1 Implement `ModelProvider` trait

1. In `clido-providers/src/provider.rs`:

   ```rust
   #[async_trait::async_trait]
   pub trait ModelProvider: Send + Sync {
       async fn complete(
           &self,
           messages: &[Message],
           tools: &[ToolSchema],
           config: &AgentConfig,
       ) -> Result<ModelResponse>;

       async fn complete_stream(
           &self,
           messages: &[Message],
           tools: &[ToolSchema],
           config: &AgentConfig,
       ) -> Result<impl Stream<Item = Result<StreamEvent>>>;
   }
   ```

2. Define `ToolSchema`:

   ```rust
   pub struct ToolSchema {
       pub name: String,
       pub description: String,
       pub input_schema: serde_json::Value,  // JSON Schema object
   }
   ```

3. Define `StreamEvent`:

   ```rust
   pub enum StreamEvent {
       TextDelta(String),
       ToolUseStart { id: String, name: String },
       ToolUseDelta { id: String, partial_json: String },
       ToolUseEnd { id: String },
       MessageDelta { stop_reason: StopReason, usage: Usage },
   }
   ```

#### 2.1.2 Implement Anthropic HTTP client

1. In `clido-providers/src/anthropic.rs`:
   - Define `AnthropicProvider` struct holding `reqwest::Client`, `api_key: String`, `model: String`.
   - Implement `new(api_key: String, model: String) -> Self`.
   - Implement `complete()`:
     - Build request body JSON:

       ```json
       {
         "model": "...",
         "max_tokens": 4096,
         "system": "...",
         "messages": [...],
         "tools": [...]
       }
       ```

     - Map `Message` → Anthropic API format (handle `ToolResult` blocks as `user` role content).
     - Map `ToolSchema` → Anthropic tool definition format.
     - POST to `https://api.anthropic.com/v1/messages`.
     - Set headers: `x-api-key`, `anthropic-version: 2023-06-01`, `content-type: application/json`.
     - Deserialize response into `ModelResponse`.
   - Return `ClidoError::Provider` on non-200 status with response body.
2. Write a unit test mocking the HTTP response with `wiremock` or `httpmock`.
3. Add integration test behind `#[cfg(feature = "integration")]` that calls real API.

#### 2.1.3 Wire provider into CLI (hardcoded)

1. In `clido-cli/src/main.rs` (temporary PoC wiring):
   - Read `ANTHROPIC_API_KEY` from environment using `std::env::var`.
   - Construct `AnthropicProvider`.
   - Construct a single `AgentConfig` with defaults.
   - Hard-code the system prompt to `"You are a helpful coding assistant."`.
2. Run `cargo check -p clido-cli`.

---

### Milestone 2.2 — Minimal Tool: Bash

**Dependency:** 2.1 complete.

#### 2.2.1 Define `Tool` trait

1. In `clido-tools/src/lib.rs`:

   ```rust
   #[async_trait::async_trait]
   pub trait Tool: Send + Sync {
       fn name(&self) -> &str;
       fn description(&self) -> &str;
       fn schema(&self) -> serde_json::Value;  // JSON Schema for input
       fn is_read_only(&self) -> bool { false }
       async fn execute(&self, input: serde_json::Value) -> ToolOutput;
   }
   pub struct ToolOutput {
       pub content: String,
       pub is_error: bool,
   }
   ```

2. Export `Tool`, `ToolOutput` from `clido-tools/src/lib.rs`.

#### 2.2.2 Implement `BashTool`

1. In `clido-tools/src/bash.rs`:
   - Struct `BashTool` with no fields (initially).
   - `name()` → `"Bash"`.
   - `description()` → `"Execute a shell command and return stdout/stderr."`.
   - `is_read_only()` → `false`.
   - `schema()` → JSON Schema with `command` (string, required), `timeout` (integer, optional), `description` (string, optional).
   - `execute()`:
     - Parse `command: String` from `input["command"]`.
     - Parse `timeout_ms: u64` from `input["timeout"].as_u64().unwrap_or(30_000)`.
     - Spawn `tokio::process::Command::new("sh").arg("-c").arg(&command)`.
     - Set timeout using `tokio::time::timeout(Duration::from_millis(timeout_ms), ...)`.
     - Capture stdout and stderr.
     - On exit code 0: return `ToolOutput { content: stdout, is_error: false }`.
     - On non-zero exit: return `ToolOutput { content: format!("Exit code {}\n{}", code, stderr), is_error: true }`.
     - On timeout: return `ToolOutput { content: "Command timed out", is_error: true }`.
2. Test with `echo hello` and `exit 1` and `sleep 100` (with short timeout).

#### 2.2.3 Implement `ToolRegistry`

1. In `clido-tools/src/registry.rs`:
   - `ToolRegistry` holding `HashMap<String, Box<dyn Tool>>`.
   - `fn register(&mut self, tool: impl Tool + 'static)`.
   - `fn get(&self, name: &str) -> Option<&dyn Tool>`.
   - `fn schemas(&self) -> Vec<ToolSchema>` — iterates all tools, calls `.schema()`.
2. In `clido-cli` PoC: construct registry, register `BashTool`.

---

### Milestone 2.3 — Minimal Agent Loop

**Dependency:** 2.1, 2.2 complete.

#### 2.3.1 Implement single-turn agent loop (PoC version)

1. In `clido-agent/src/loop.rs` create `AgentLoop` struct:

   ```rust
   pub struct AgentLoop {
       provider: Arc<dyn ModelProvider>,
       tools: ToolRegistry,
       config: AgentConfig,
       history: Vec<Message>,
   }
   ```

2. Implement `AgentLoop::new(provider, tools, config)`.
3. Implement `async fn run(&mut self, user_input: &str) -> Result<String>`:
   - Push `Message { role: User, content: [Text(user_input)] }` to `self.history`.
   - Loop (up to `config.max_turns`):
     - Get tool schemas from registry.
     - Call `provider.complete(&self.history, &schemas, &self.config)`.
     - Push the assistant `ModelResponse` content into history as assistant message.
     - If `stop_reason == EndTurn` or no `ToolUse` blocks: break, return text content.
     - For each `ToolUse` block in response:
       - Look up tool by name in registry.
       - If not found: push `ToolResult { is_error: true, content: "Tool not found: ..." }`.
       - Else: call `tool.execute(input).await`.
       - Push resulting `ToolResult` as `user` message content block.
   - After loop: return final text or error.
4. Handle `max_turns` exceeded: return `ClidoError::Other("max_turns exceeded")`.

#### 2.3.2 Wire the PoC CLI

1. In `clido-cli/src/main.rs`:
   - Parse `--print`/`-p` flag using a simple `std::env::args()` loop (not clap yet).
   - Instantiate `AnthropicProvider`, `ToolRegistry` with `BashTool`, `AgentConfig` with defaults.
   - Instantiate `AgentLoop`.
   - Call `loop.run(user_input).await`.
   - Print result to stdout.
2. Run end-to-end PoC: `ANTHROPIC_API_KEY=... cargo run -p clido-cli -- "list files in current directory"`.
3. Verify the loop executes at least one tool call and returns text.

#### 2.3.3 PoC validation checklist

- [ ] Model is called with correct message format
- [ ] Tool schema is sent to model
- [ ] Model requests `Bash` tool
- [ ] Bash tool executes the command
- [ ] Result is sent back to model as `tool_result`
- [ ] Model returns final text response
- [ ] Total token usage is logged at debug level
- [ ] Non-zero exit codes produce `is_error: true`

---

## Phase 3 — Minimal Viable Agent

**Goal:** Complete tool set, proper CLI with clap, session storage, configuration loading.
**Exit criteria:** `clido "audit this repository"` completes a multi-step task on a real repo with all six core tools available.

---

### Milestone 3.1 — Complete Tool Set

**Dependency:** Phase 2 complete.

#### 3.1.1 Implement `ReadTool`

1. In `clido-tools/src/read.rs`:
   - `name()` → `"Read"`.
   - `is_read_only()` → `true`.
   - Parameters: `file_path` (string, required), `offset` (integer, optional, 1-based line number), `limit` (integer, optional, number of lines).
   - Implementation:
     - Read file with `tokio::fs::read_to_string(&file_path).await`.
     - If `offset` or `limit` specified: collect lines, slice `[offset-1 .. offset-1+limit]`.
     - Prefix each line with `"     N→"` format (right-aligned 6-char field, `→` separator) matching observed Claude format.
     - Return full prefixed content as `ToolOutput`.
   - Error cases: file not found → `is_error: true`, `"File does not exist. Note: your current working directory is {cwd}."`.
   - Error case: path is a directory → `is_error: true`, `"EISDIR: illegal operation on a directory, read '{path}'"`.
2. Tests:
   - Read a fixture file without offset/limit: verify line prefix format.
   - Read with offset=3, limit=5: verify correct slice.
   - Read non-existent file: verify is_error and message contains cwd.
   - Read directory path: verify EISDIR error.

#### 3.1.2 Implement `WriteTool`

1. In `clido-tools/src/write.rs`:
   - `is_read_only()` → `false`.
   - Parameters: `file_path` (string, required), `content` (string, required).
   - Implementation:
     - Create parent directories with `tokio::fs::create_dir_all(parent)`.
     - Write with `tokio::fs::write(&file_path, &content).await`.
     - Return `"File written successfully."` on success.
   - Error: return `is_error: true` with IO error message.
2. Tests:
   - Write a new file: verify contents on disk.
   - Write to a nested path that doesn't exist: verify parent dirs created.
   - Write to a read-only path: verify is_error.

#### 3.1.3 Implement `EditTool`

1. In `clido-tools/src/edit.rs`:
   - `is_read_only()` → `false`.
   - Parameters: `file_path`, `old_string`, `new_string`, `replace_all` (boolean, default false).
   - Implementation:
     - Read file content.
     - If `replace_all`: use `content.replace(&old_string, &new_string)`.
     - Else: replace first occurrence only. If `old_string` not found → `is_error: true`, `"<tool_use_error>String to replace not found in file.\nString: {old_string}</tool_use_error>"` (exact format from traces).
     - Write updated content back.
     - Return `"The file {path} has been updated successfully."` on success.
   - Generate a unified diff patch using `similar` crate; store as `toolUseResult` metadata in session recording.
2. Tests:
   - Edit a known string: verify file updated.
   - Edit with replace_all=true: verify all occurrences replaced.
   - Edit with string not found: verify is_error and exact error format.
   - Edit to empty string (deletion): verify works.

#### 3.1.4 Implement `GlobTool`

1. In `clido-tools/src/glob.rs`:
   - `is_read_only()` → `true`.
   - Parameters: `pattern` (string, required), `path` (string, optional, defaults to cwd).
   - Implementation:
     - Use the `ignore` crate (`WalkBuilder`) or the `glob` crate.
     - Walk from `path`, match entries against `pattern`.
     - Sort results by modification time descending (matches Claude behavior).
     - Return newline-joined list of matching paths.
2. Tests:
   - Glob `**/*.rs` in workspace: verify expected files found.
   - Glob with specific directory path: verify scoped results.
   - Glob on non-existent path: verify is_error.

#### 3.1.5 Implement `GrepTool`

1. In `clido-tools/src/grep.rs`:
   - `is_read_only()` → `true`.
   - Parameters (from Cursor bundle analysis and observed error messages):
     - `pattern` (string, required)
     - `path` (string, optional)
     - `output_mode` (string, optional): `"content"` | `"files_with_matches"` | `"count"` — default `"files_with_matches"`.
     - `context` (integer, optional): lines before and after match.
     - `-A` (integer): lines after.
     - `-B` (integer): lines before.
     - `-C` (integer): alias for context.
     - `-i` (boolean): case insensitive.
     - `-n` (boolean): include line numbers.
     - `glob` (string): filter files by glob.
     - `type` (string): file type filter (e.g. `"rs"`, `"js"`).
     - `multiline` (boolean): multiline mode.
     - `head_limit` (integer): limit output lines.
   - Implementation: use the `grep` crate (part of ripgrep family) for matching, or spawn `rg` subprocess if available.
   - Input validation: reject unknown parameters with `is_error: true`, `"InputValidationError: Grep failed due to the following issue:\nAn unexpected parameter \`{key}\` was provided"` (exact format from traces).
2. Tests:
   - Search for a known string in a fixture directory.
   - Test `output_mode: "files_with_matches"`.
   - Test `output_mode: "content"` with context=2.
   - Test case-insensitive match.
   - Test unknown parameter → InputValidationError.

#### 3.1.6 Register all tools in CLI

1. In `clido-cli/src/main.rs`:
   - Build `ToolRegistry` with all six tools: `ReadTool`, `WriteTool`, `EditTool`, `GlobTool`, `GrepTool`, `BashTool`.
2. Integration test: run agent with `"show me the first 5 lines of Cargo.toml"` — verify Read tool used.

---

### Milestone 3.2 — Proper CLI with `clap`

**Dependency:** 3.1 complete.

#### 3.2.1 Replace PoC arg parsing with `clap`

**Reference:** Full CLI surface (all flags, subcommands, precedence, conflicts) is defined in [cli-interface-specification.md](cli-interface-specification.md). The struct below should include every V1 flag from the spec; extend with V1.5/V2 flags as those releases are implemented.

1. In `clido-cli/src/main.rs`, define `Cli` struct with `clap::Parser`:

   ```rust
   #[derive(Parser, Debug)]
   #[command(name = "clido", version, about = "Local-first CLI coding agent")]
   struct Cli {
       /// Task to execute (positional)
       prompt: Option<String>,
       /// Non-interactive / print mode
       #[arg(short = 'p', long)]
       print: bool,
       /// Output format: text (default), json, stream-json
       #[arg(long, default_value = "text")]
       output_format: OutputFormat,
       /// Use a named profile from config (e.g. cheap, review, local)
       #[arg(long)]
       profile: Option<String>,
       /// Provider override: anthropic | openai | openrouter | local
       #[arg(long)]
       provider: Option<String>,
       /// Model override (e.g. claude-sonnet-4-5 or openrouter/anthropic/claude-haiku-3-5)
       #[arg(long)]
       model: Option<String>,
       /// Resume a previous session
       #[arg(long)]
       resume: Option<String>,
       /// Maximum number of turns
       #[arg(long)]
       max_turns: Option<u32>,
       /// Maximum cost in USD
       #[arg(long)]
       max_budget_usd: Option<f64>,
       /// System prompt override
       #[arg(long)]
       system_prompt: Option<String>,
       /// Permission mode: default, accept-all, plan
       #[arg(long, default_value = "default")]
       permission_mode: PermissionMode,
       /// Allowed tools (comma-separated)
       #[arg(long)]
       allowed_tools: Option<String>,
      /// Disallowed tools (comma-separated)
      #[arg(long)]
      disallowed_tools: Option<String>,
      /// Tools allowlist: only these tools are registered (comma-separated). Distinct from allowed_tools (which controls prompting).
      #[arg(long)]
      tools: Option<String>,
      /// System prompt from file (replaces system_prompt if both set)
      #[arg(long)]
      system_prompt_file: Option<PathBuf>,
      /// Append to system prompt (after file or --system-prompt)
      #[arg(long)]
      append_system_prompt: Option<String>,
      /// Resume newest session for current project (cannot be used with --resume)
      #[arg(long)]
      continue_: bool,
      /// Disable color and Unicode-heavy output
      #[arg(long)]
      no_color: bool,
      /// Verbose: full tool I/O, timestamps
      #[arg(short = 'v', long)]
      verbose: bool,
      /// (V1.5+) Quiet: suppress spinner, tool lifecycle, cost footer
      #[arg(short = 'q', long)]
      quiet: bool,
      /// (V2+) Input format: text | stream-json (automation-only)
      #[arg(long, default_value = "text")]
      input_format: InputFormat,
      /// (V1.5+) MCP config file path
      #[arg(long)]
      mcp_config: Option<PathBuf>,
  }
  ```

  Support `--version` and `clido version` subcommand per spec (D1). Add `OutputFormat`, `PermissionMode`, and `InputFormat` as `clap::ValueEnum`. Env var equivalents (e.g. `CLIDO_MODEL`, `CLIDO_PROFILE`, `NO_COLOR`) are documented in the CLI spec; resolve from env when the flag is not set.

2. Add `OutputFormat` and `PermissionMode` as `clap::ValueEnum`.
3. Wire parsed flags into `AgentConfig`.
4. Read prompt from stdin if not provided as arg and not `--print` mode (interactive REPL stub).
5. Test: `clido --help` prints usage; `clido -p "hello"` runs.

#### 3.2.2 Add interactive REPL mode

1. If no prompt given and not `--print`: enter interactive mode.
2. Print `clido> ` prompt, read line from stdin with `rustyline` crate (for history/editing).
3. On each input, run agent loop with preserved history.
4. Exit on Ctrl-C or `exit`/`quit` input.
5. Test: `clido` without args enters REPL; multi-turn conversation works.

#### 3.2.3 Implement session subcommands (canonical: `sessions list` / `show` / `fork`)

**Reference:** Session command UX (output format, ID display, `--json`) is defined in [cli-interface-specification.md](cli-interface-specification.md) §9.

1. Add `#[command(subcommand)] command: Option<Commands>` to `Cli`.
2. Define canonical subcommands: `Sessions(SessionsCmd)` with `SessionsCmd::List`, `Show { session_id }`, `Fork { session_id }` (fork in V1.5). Use noun-first grouping: `clido sessions list`, `clido sessions show <id>`, `clido sessions fork <id>`. Legacy aliases `list-sessions` and `show-session` remain as hidden aliases with a deprecation notice to stderr (see spec §16).
3. Implement in `clido-cli/src/commands/sessions.rs` (or list_sessions.rs):
   - Read session directory (see 3.3).
   - Print session_id (first 8 + `...` + last 4 chars), first message preview (50 chars), timestamp, turn count, cost. Sort newest first.

---

### Milestone 3.3 — Session Storage

**Dependency:** 3.2 complete.

#### 3.3.1 Define session data structures

1. In `clido-storage/src/session.rs`:
   - `SessionEnvelope`:

     ```rust
     pub struct SessionEnvelope {
         pub session_id: String,
         pub created_at: DateTime<Utc>,
         pub project_path: PathBuf,
         pub messages: Vec<SessionLine>,
     }
     ```

   - `SessionLine` — wraps a type-tagged line matching JSONL format:

     ```rust
     #[serde(tag = "type", rename_all = "snake_case")]
     pub enum SessionLine {
         User { message: Message },
         Assistant { message: Message },
         Result {
             subtype: String,
             duration_ms: u64,
             is_error: bool,
             num_turns: u32,
             total_cost_usd: f64,
             usage: Usage,
             result: Option<String>,
         },
         System { subtype: String },
         Progress { data: serde_json::Value },
     }
     ```

#### 3.3.2 Implement session write (JSONL)

1. Compute session directory: `{data_dir}/projects/{sanitized_cwd}/{session_id}.jsonl`.
   - Use `directories::ProjectDirs::from("", "", "clido")` → `data_dir()`.
   - Sanitize cwd: replace `/` with `-`, strip leading `-`.
2. Implement `SessionWriter`:
   - Opens/creates JSONL file on first write.
   - `fn append(&mut self, line: &SessionLine)` — serializes to JSON, writes line + `\n`.
   - Uses `tokio::fs::OpenOptions` with append mode.
3. Write to session file after each user message, each assistant response, and after result.

#### 3.3.3 Implement session read (resume)

1. Implement `SessionReader::load(session_id: &str, project_path: &Path) -> Result<Vec<SessionLine>>`:
   - Locate JSONL file by session_id.
   - Read lines, deserialize each, collect `SessionLine`s.
   - Reconstruct `Vec<Message>` from user/assistant lines (skip result/system/progress).
2. Wire `--resume {session_id}` in CLI: load session, prepend history to new `AgentLoop`.
3. Test: run task → kill process → resume with session_id → verify history is restored and agent continues.

#### 3.3.4 Implement session directory discovery

1. `fn list_sessions(project_path: &Path) -> Result<Vec<SessionSummary>>`:
   - Enumerate `*.jsonl` files in project session dir.
   - For each: read first user line for preview, get file modified time.
   - Return sorted by modified time descending.
2. Wire into `clido sessions list` (and legacy `clido list-sessions` with deprecation notice).

---

### Milestone 3.4 — Configuration Loading

**Dependency:** 3.3 complete.

#### 3.4.1 Load configuration from file

**Config loading order** (each level overrides the previous):
1. Built-in defaults
2. Global user config: `~/.config/clido/config.toml`
3. Project config: `.clido/config.toml` in the current working directory (or the nearest ancestor directory containing it — walk upward, stop at home dir or fs root)
4. CLI flags (highest priority)

This means a repo can have its own `.clido/config.toml` that selects a different provider, model, or profile for that project without touching the global config.

**Global config location:** `~/.config/clido/config.toml` (via `directories`).

**Config schema:**

```toml
# ---
# Default profile active for this machine/repo.
# Override per-invocation with: clido --profile cheap
default_profile = "default"

# ---
# Named profiles. Each maps a name to a provider + model selection.
# Use --profile <name> on the CLI to activate a profile for one run.
[profile.default]
provider = "anthropic"
model = "claude-sonnet-4-5"
api_key_env = "ANTHROPIC_API_KEY"    # env var to read key from
base_url = "https://api.anthropic.com"

[profile.cheap]
provider = "openrouter"
model = "anthropic/claude-haiku-3-5"
api_key_env = "OPENROUTER_API_KEY"
base_url = "https://openrouter.ai/api/v1"

[profile.local]
provider = "local"
model = "codellama"
base_url = "http://localhost:11434/v1"  # Ollama

[profile.review]
provider = "anthropic"
model = "claude-opus-4-6"
api_key_env = "ANTHROPIC_API_KEY"

# ---
# Agent defaults (apply to all profiles unless overridden by CLI flags).
[agent]
max_turns = 50
max_budget_usd = 5.0

# ---
[tools]
allowed = []               # empty = all allowed
disallowed = ["Bash"]      # example
```

**Project-level config** (`.clido/config.toml` in cwd or ancestor):
- Uses the same schema.
- Only the fields it specifies are merged; the rest inherit from the global config.
- Example: a project that always uses OpenRouter + Sonnet only needs:

  ```toml
  default_profile = "project-model"

  [profile.project-model]
  provider = "openrouter"
  model = "anthropic/claude-sonnet-4-5"
  api_key_env = "OPENROUTER_API_KEY"
  ```

**Config::load():**
1. Check `$CLIDO_CONFIG`; if set, use that path as the global config.
2. Otherwise: `~/.config/clido/config.toml`.
3. Parse global config with `toml` crate. Fall back to empty defaults if file absent.
4. Walk from cwd upward (stop at `$HOME` or fs root) to find `.clido/config.toml`. If found, merge it: project config values override global config values.
5. Merge: CLI flags override merged config.

**API key validation:**
When the selected profile's `api_key_env` is set but the corresponding environment variable is empty or absent, emit a clear, actionable error at startup (not an obscure HTTP error at first API call):
```
Error: API key not found for profile 'default'.
Set the environment variable ANTHROPIC_API_KEY, or configure 'api_key_env' in
~/.config/clido/config.toml or .clido/config.toml.
```

**Tests:**
- Load config with no file present → verify defaults are used.
- Load global config + project config → verify project values override global.
- Select a non-existent profile → verify clear error message.
- Select a profile with missing API key env var → verify helpful error at startup.
- CLI `--profile cheap` → verify that profile's provider/model are active.

#### 3.4.2 Load project instructions (`CLIDO.md` or `CLAUDE.md`)

1. In `clido-context/src/project.rs`:
   - Walk from cwd upward to find `CLIDO.md` or `CLAUDE.md`.
   - Read content; treat as additional system prompt prefix.
   - **Trust-on-first-use:** Maintain a small allowlist of (canonical path → hash) for project instruction files that the user has already approved (e.g. in `{data_dir}/trusted_project_instructions.json`). When loading an `CLIDO.md` or `CLAUDE.md` that is not on the allowlist (or whose content hash has changed), prompt the user once: `"Load project instructions from {path}? [y/N]"`. If the user confirms, add the path and hash to the allowlist. If not, skip loading project instructions for this run. In non-interactive mode, skip loading unless the path is already trusted.
   - **Size limit:** Enforce a maximum size on project instructions (e.g. ~4,000 tokens using the same token counter as context). If the file exceeds the limit, truncate with a trailing note `"[Project instructions truncated at {limit} tokens.]"` and log a warning. This limits both cost and the impact of adversarial or accidental huge files.
   - Notify user when project instructions are loaded.
2. Prepend project instructions to system prompt in context builder.
3. Test: create `CLIDO.md` in fixture dir, verify it's included in system prompt.
4. Test: load from an untrusted path (not in allowlist); verify prompt is shown; confirm and verify path is trusted on next run.
5. Test: create `CLIDO.md` larger than token limit; verify truncation and warning.

#### 3.4.3 Inject tool usage guidance into system prompt

1. In `clido-context/src/guidance.rs`:
   - Define `fn build_tool_guidance(registry: &ToolRegistry) -> String`.
   - Output a structured block appended to the system prompt:

     ```
     ## Tool Usage Guidelines

     Use these tools to accomplish tasks:

     - **Glob**: Explore repository structure. Example: Glob(pattern="**/*.rs", path=".")
     - **Read**: Inspect file contents. Use offset/limit for large files.
     - **Grep**: Search for patterns across files. Use output_mode="content" with context for
       code search.
     - **Bash**: Run shell commands. Prefer read-only commands (git log, cargo check) before
       writing.
     - **Edit**: Modify existing files using exact string replacement.
       Always Read the file first to get the exact old_string.
     - **Write**: Create new files or fully replace file contents.

     Exploration pattern: Glob → Read → Grep → Read → Edit/Write
     ```

2. Append tool guidance block to system prompt in `ContextBuilder::build()`.
3. Make guidance generation dynamic: regenerate if tool registry changes (e.g. MCP tools added).
4. Test: build context with known registry → verify guidance block appears in system message.

---

### Milestone 3.5 — Streaming Output

**Dependency:** 3.2, 3.4 complete.

#### 3.5.1 Implement Anthropic streaming

1. Add `stream: true` to Anthropic API request.
2. Use `reqwest` streaming response with `bytes_stream()`.
3. Parse SSE (`text/event-stream`) line-by-line:
   - `event: content_block_delta` with `delta.type: "text_delta"` → emit `TextDelta`.
   - `event: content_block_start` with `type: "tool_use"` → emit `ToolUseStart`.
   - `event: content_block_delta` with `delta.type: "input_json_delta"` → emit `ToolUseDelta`.
   - `event: content_block_stop` → emit `ToolUseEnd`.
   - `event: message_delta` with `usage` → emit `MessageDelta`.
4. Assemble partial tool JSON in `ToolUseDelta`: maintain a `HashMap<id, String>` buffer; append each `partial_json` fragment; on `ToolUseEnd` parse the accumulated string into `serde_json::Value`.
5. Return `impl Stream<Item = Result<StreamEvent>>` from provider.
6. Update `ModelProvider::complete_stream()` method signature.

#### 3.5.2 Wire streaming to CLI output

1. In `clido-cli/src/output/streaming.rs`:
   - For `text` output format: print `TextDelta` chunks directly to stdout without newline.
   - For `stream-json` format: print each event as a JSON object on its own line.
   - For `json` format: buffer everything, print final JSON object.
2. Show tool call status lines during execution: `"⏳ Bash: ls -la"` → `"✓ Bash: ls -la"`.
3. Test: streaming output appears incrementally in terminal.

---

## Phase 4 — Feature Expansion

**Goal:** Multi-provider support, context compaction, prompt caching, permission system, subagents, bounded concurrency, task graph planner (optional), plan mode.
**Exit criteria:** All providers work; context compaction triggers on long sessions; prompt caching is active on Anthropic (verified via `cache_read_input_tokens > 0` in multi-turn sessions); permission prompts work; subagents can run in parallel; bounded concurrency prevents runaway tool calls.

---

### Milestone 4.1 — Multi-Provider Support

**Dependency:** Phase 3 complete.

#### 4.1.1 Implement OpenAI provider

1. In `clido-providers/src/openai.rs`:
   - Struct `OpenAIProvider` with `client`, `api_key`, `base_url`, `model`.
   - Map `Message`/`ContentBlock` to OpenAI chat completions format:
     - `tool_use` → `assistant` message with `tool_calls` array.
     - `tool_result` → `tool` role message with `tool_call_id` and `content`.
   - Map `ToolSchema` → OpenAI function format: `{ "type": "function", "function": { "name": ..., "description": ..., "parameters": ... } }`.
   - POST to `/v1/chat/completions`.
   - Map response back to `ModelResponse`.
2. Test: mock HTTP server, verify request format and response parsing.

#### 4.1.2 Implement OpenRouter provider

1. In `clido-providers/src/openrouter.rs`:
   - OpenRouter uses OpenAI-compatible API at `https://openrouter.ai/api/v1`.
   - Reuse `OpenAIProvider` implementation with configurable `base_url`.
   - Add `HTTP-Referer` and `X-Title` headers (OpenRouter requirements).
   - Support model name as passed (e.g. `anthropic/claude-opus-4-6`).
2. Test: verify headers are sent.

#### 4.1.3 Implement local model provider (Ollama)

1. In `clido-providers/src/local.rs`:
   - Target: Ollama at `http://localhost:11434/v1` (OpenAI-compatible endpoint).
   - Tool call support: depends on model. Implement best-effort JSON parsing for function calling.
   - Fallback: if model doesn't support tool use natively, inject tool schemas into system prompt as JSON and parse tool calls from text output.
2. Test: integration test with a running Ollama instance (behind feature flag).

**Offline / air-gapped mode:**

Clido is designed to be local-first. Users running Ollama (or another local provider) should be able to operate Clido with zero internet access. Define this explicitly:

- **Config key:** `[provider] offline = true` (or equivalently, use a profile with `provider_type = "ollama"`). When `offline = true`, Clido must **never** attempt to reach any external host. This includes:
  - Model API calls (all go to `localhost` only).
  - Telemetry/analytics pings — disabled completely.
  - `clido update-pricing` — skipped with a message (no network call attempted).
  - `fastembed` model download (Phase 5.5.3) — if the ONNX model cache is absent in offline mode, fail with a clear message: `"Embedding model not cached. Run 'clido fetch-models' with network access first, then switch to offline mode."` Do not silently fail or produce wrong embeddings.
  - `clido doctor` connectivity check — skip the API ping; mark as `[skipped — offline mode]` not `✗`.
- **Startup check in offline mode:** If `offline = true` and the selected provider is not local (i.e. has an `api_base_url` that is not `localhost` or `127.0.0.1`), emit an error at startup: `"offline = true but provider points to '{url}'. Set provider_type to 'ollama' or configure a localhost base_url."`.
- **CI:** Tests that use mock HTTP servers (wiremock) are fine; tests that require real network access must be guarded with `#[cfg(feature = "online-tests")]` and excluded from the default CI matrix.
- Document the offline mode in the user README under "Offline / Air-gapped Usage."

#### 4.1.4 Implement Alibaba Cloud (Qwen) provider

1. In `clido-providers/src/alibaba.rs`:
   - Alibaba Cloud DashScope API: `https://dashscope.aliyuncs.com/compatible-mode/v1` (OpenAI-compatible).
   - Add `Authorization: Bearer {api_key}` header.
   - Support Qwen model names (e.g. `qwen-max`, `qwen-plus`).
2. Reuse `OpenAIProvider` with `base_url` override.

#### 4.1.5 Provider factory

1. In `clido-providers/src/lib.rs`:
   - `fn build_provider(config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>>`:
     - Resolve the active profile from config (using `--profile` flag or `default_profile`).
     - Extract `provider_type`, `model`, and `api_key_env` from the resolved profile.
     - If `--provider` CLI flag is set, override `provider_type`. If `--model` flag is set, override `model`.
     - **Unsupported provider guard:** If the requested `provider_type` is not implemented in the current binary, return a clear `ClidoError::Config` immediately, **not** a runtime HTTP error. Example: in V1, only `Anthropic` is available; selecting `openrouter` returns `"Provider 'openrouter' is not yet supported in this version. Available: anthropic."`. This matters because V1 ships with a full profile config schema — users may configure profiles for providers that only become available in V2.
     - Read API key from the environment variable named by `api_key_env`. If the variable is missing or empty, return a helpful `ClidoError::Config` (see validation in 3.4.1) rather than proceeding to get an opaque HTTP 401.
     - **Model tool-use compatibility check:** After building the provider, make a cheap validation call or use a static lookup to verify the selected model supports tool use. If the model is known not to support tool use (e.g. most local Ollama models), emit a prominent warning at startup: `"Warning: model '{model}' may not support tool use. Clido requires tool use to function. Continuing, but expect errors."` Do not abort — local models vary; emit the warning and proceed. Document which models are known-compatible in the user docs.
2. Wire into CLI config loading.
3. All HTTP-based providers must use the shared retry/backoff logic from Phase 5.1.0 when making API calls (so rate limits and 5xx are handled consistently).
4. Test: provider factory returns correct type for each config variant.
5. Test: resolve profile `cheap` → verify OpenRouter provider and haiku model are selected.
6. Test: `--profile review --model claude-haiku-3-5` → verify model override wins over profile.
7. Test: profile with missing API key env var → verify `ClidoError::Config` with actionable message.
8. Test: V1 binary with profile pointing to `openrouter` → verify `ClidoError::Config("not yet supported")` not a runtime panic.
9. Test: model known not to support tool use → verify warning is emitted at startup, agent still starts.

---

### Milestone 4.2 — Context Engine

**Dependency:** Phase 3 complete.

#### 4.2.1 Implement token counter

1. In `clido-context/src/compaction.rs`:
   - Use `tiktoken-rs` crate for cl100k/o200k token counting as a rough estimate.
   - Note: Claude uses a different tokenizer; counts are approximate (±10%). Use conservatively.
   - Implement `fn count_tokens(messages: &[Message]) -> usize`.
   - Serialize each message to its JSON API format, then count tokens.
   - **Safety margin:** Because of the ±10% error, the effective context limit used for compaction must leave headroom. Either reserve a fixed `context_overflow_safety_margin` (e.g. 1000 tokens) that is never used for input content, or use a compaction threshold that is conservative (see 4.2.2). This avoids context overflow API errors when the true token count is higher than the estimate.
2. Test: known text → known token count (within ±10%).

#### 4.2.2 Implement context builder

1. In `clido-context/src/builder.rs`:

   ```rust
   pub struct ContextBuilder {
       max_tokens: usize,
       /// Compaction triggers when context reaches this fraction of max_tokens. Default 0.75 (not 0.85)
       /// to account for tiktoken ±10% error and leave headroom for the next model response.
       compaction_threshold: f64,
       /// Optional: tokens always reserved (never used for input). Further reduces overflow risk.
       context_overflow_safety_margin: usize,
   }
   impl ContextBuilder {
       pub fn build(&self, config: &AgentConfig, history: &[Message]) -> Vec<Message>
   }
   ```

2. `build()` logic:
   - Effective limit = `max_tokens - context_overflow_safety_margin` (e.g. default margin 1000).
   - Include system prompt as first message (or as API `system` field).
   - Walk history from oldest to newest; count tokens as you add.
   - If adding the next message would exceed `effective_limit * compaction_threshold`: trigger compaction. Default `compaction_threshold` to 0.75 so that even with 10% undercount we stay well below provider limits.
   - Otherwise: include all messages.

**Pinned messages (definition):** A message is **pinned** if and only if it is a `system`-role message injected by the context builder (e.g. project instructions from CLIDO.md, tool usage guidance, or other builder-injected system content). User messages are never pinned. The compaction pass skips pinned messages when selecting the "oldest N" block to summarize; after generating the summary, it re-inserts all pinned messages verbatim above the summary block so they remain in full in the context. There is no user-facing command to pin arbitrary messages; pinning is automatic and limited to builder-injected content. See [devdocs/algorithms/context-compaction.md](../algorithms/context-compaction.md).

#### 4.2.3 Implement context compaction

**Output UX:** Compaction must be visible to the user (never silent). Rich TTY: transient message (e.g. "↻ Compacting context…" then "✓ Context compacted"); ASCII/non-TTY: persistent lines `[compact] …`. See [cli-interface-specification.md](cli-interface-specification.md) §5 (D10). Emit `compact_start` / `compact_end` in `--output-format stream-json`.

1. **Pinned messages:** Apply the definition above. Only system-injected messages (project instructions, tool guidance) are pinned and preserved verbatim; they are prepended to the compacted summary.
2. When context exceeds threshold:
   - Take the oldest N **non-pinned** messages (those that won't fit).
   - Call a secondary "summarize" request to the model: `"Summarize the following conversation history in 3-5 sentences, focusing on decisions made and context established: {oldest_messages}"`.
   - **Summarization failure fallback:** The summarization call can fail (provider error, model refusal, timeout). Define explicit failure handling:
     - On provider error (HTTP 5xx, timeout, rate limit after retries): **do not compact**. Continue with the full overflowing context. Log a `warn!` event: `"Context compaction failed; proceeding with full context (context_tokens={n}, limit={limit})."`. This is safe — the model receives slightly more context than ideal, but does not lose information.
     - On model refusal or empty summary returned: treat as provider error (do not compact).
     - If context is so large that not compacting would produce a request exceeding the model's hard limit (i.e. `context_tokens > hard_limit`): emit a user-visible error and halt the turn rather than silently sending a request that will be rejected. The error should read: `"Context is too large to compact and too large to send ({n} tokens > {hard_limit}). Start a new session or use --context-limit to configure a smaller limit."`.
   - On success: replace those oldest messages with a single `system` message: `"[Compacted history] {summary}"`. Then prepend any pinned messages so they appear in full above the summary.
   - Emit a `SessionLine::System { subtype: "compact_boundary" }` to the session file.
3. Test: create a fixture with a very long conversation; verify compaction triggers and summary is included.
4. Test: include an early user message with "IMPORTANT: never modify src/prod/"; run until compaction; verify that constraint still appears verbatim in the context (or that pinned messages survive compaction).
5. Test: mock summarization provider to return an error → verify compaction is skipped, agent continues, `warn!` is emitted.
6. Test: context exceeds hard model limit and summarization fails → verify user-visible error message and halted turn (not a panic).
7. **Measurable compaction criterion:** The compaction summary must contain the file path of every file successfully read or edited in the compacted turns, and the name of every tool called in those turns. Test: run a fixture with exactly 5 known tool calls across 3 known files; trigger compaction; verify the summary text contains all 3 file paths and all 5 tool names (e.g. via string assertion or regex). This makes "retains enough context" testable.

---

### Milestone 4.2.4 — Prompt Caching (Anthropic)

**Dependency:** 4.2.1 complete, 4.1 (Anthropic provider) complete.

**Rationale:** Anthropic's prompt caching bills cached input tokens at 10% of the normal price with up to 85% lower latency on long prompts. In a multi-turn agent session, the system prompt, tool definitions, and CLIDO.md content are sent with every API call. Without caching, these tokens are billed at full price every turn. With caching, they are billed at 1/10th price after the first call. On a 20-turn session with a 10K-token system prompt, this reduces input costs by 60–80%.

Anthropic also supports **automatic caching** for multi-turn conversations since 2025: the API automatically applies cache breakpoints to the last cacheable message block. The implementation below adds explicit breakpoints for the highest-value fixed content (system prompt, tools), and relies on automatic caching for the conversation history.

#### 4.2.4.1 Mark system prompt and tool definitions as cacheable

1. In `clido-providers/src/anthropic.rs`, when building the API request:
   - Add `"cache_control": {"type": "ephemeral"}` to the last text block of the system prompt content array.
   - Add `"cache_control": {"type": "ephemeral"}` to the last tool definition in the tools array.

   ```rust
   // Example: system prompt as content block array (Anthropic API format)
   let mut system_blocks = build_system_blocks(&context);
   if let Some(last) = system_blocks.last_mut() {
       last["cache_control"] = json!({"type": "ephemeral"});
   }

   // Example: tool definitions
   let mut tools = build_tool_schemas(&registry);
   if let Some(last) = tools.last_mut() {
       last["cache_control"] = json!({"type": "ephemeral"});
   }
   ```

2. Anthropic requires at least 1,024 tokens in a cacheable block for caching to activate. Log a `debug!` warning if the system prompt is shorter.

#### 4.2.4.2 Track cache metrics in Usage

1. `Usage` already has `cache_creation_input_tokens: Option<u64>` and `cache_read_input_tokens: Option<u64>` — ensure these are populated from the API response.
2. In cost tracking (4.4), apply the correct rate for cached tokens:
   - `cache_creation_input_tokens` → billed at 125% of normal (one-time write cost)
   - `cache_read_input_tokens` → billed at 10% of normal
3. Update `ModelPricing` struct to include `cache_creation_multiplier` and `cache_read_multiplier` fields.
4. Display cache read tokens in the per-turn cost summary: `"Cost: $0.0012 (↓ 73% via cache)"`.

#### 4.2.4.3 Tests

1. Mock Anthropic server: verify `cache_control` fields are present on system blocks and tool definitions.
2. Two consecutive calls with identical system prompt: verify second call has `cache_read_input_tokens > 0` in response.
3. Cost calculation: inject a usage with 1000 normal + 500 cache_read tokens → verify cost uses 10% rate for cached portion.

---

### Milestone 4.3 — Permission System

**Dependency:** 4.1 complete.

#### 4.3.1 Implement permission checker

1. In `clido-agent/src/permissions.rs`:
   - `PermissionChecker` struct with `mode: PermissionMode`, `allowed: Vec<ToolPattern>`, `disallowed: Vec<ToolPattern>`.
   - `async fn check(&self, tool_name: &str, input: &serde_json::Value) -> PermissionDecision`.
   - `PermissionDecision` enum: `Allow`, `Deny`, `AskUser`.

#### 4.3.2 Implement permission modes

1. `PermissionMode::AcceptAll` → always return `Allow`.
2. `PermissionMode::PlanOnly` → allow only: `Read`, `Glob`, `Grep` (no Bash, no Write, no Edit). Bash is blocked entirely in plan mode; see Milestone 4.5.1.
3. `PermissionMode::Default` → check allow/disallow lists; if state-changing tool not in allow list → `AskUser`.

#### 4.3.3 Implement interactive permission prompt

1. When `AskUser` returned in interactive mode:
   - Print: `"Allow tool {name} with input: {pretty_printed_input}? [y/N/a(lways)/d(isallow)]"`.
   - Read stdin response.
   - `y` → `Allow` this time; `a` → add to allow list for session; `d` → deny.
2. When in `--print` mode (non-interactive): treat `AskUser` as `Deny`, log warning.
3. **Serialized permission gate — required to prevent deadlock:**
   - The `PermissionGate` must be backed by a `Mutex<()>` (or `tokio::sync::Mutex`) that serializes all `AskUser` prompts, regardless of the number of concurrent tools or subagents in flight.
   - **Why:** In Phase 4.6, multiple read-only tools execute concurrently via `join_all`. If the batch also contains a state-changing tool that requires `AskUser`, or if a subagent (4.7) triggers a permission prompt while another is already waiting on stdin, multiple futures simultaneously attempt to read from stdin. This produces garbled output and can deadlock because a second stdin reader blocks while the first is suspended.
   - **Implementation:** Wrap stdin interaction in a `tokio::sync::Mutex<StdinPermissionPrompt>`. All `AskUser` futures acquire the lock before printing the prompt. Queued prompts block (not spin) until the current prompt is answered.
   - **Subagent interaction:** `SubAgentManager` must share the same `Arc<Mutex<StdinPermissionPrompt>>` as the parent agent. Subagents that need a permission prompt enqueue on the same gate. The user sees one prompt at a time, even when multiple subagents run in parallel.
   - **Non-interactive mode:** No change — `AskUser` immediately resolves to `Deny` without acquiring the lock.
4. Test: mock stdin with known responses; verify tool is blocked or allowed.
5. Test: two concurrent `join_all` futures each trigger `AskUser` → verify exactly one prompt appears at a time (no interleaved output), second prompt appears only after first is answered.
6. Test: two parallel subagents each trigger `AskUser` → verify serialized behavior, no deadlock within 5s timeout.

---

### Milestone 4.4 — Cost Tracking

**Dependency:** 4.1 complete.

#### 4.4.1 Track cumulative cost per session

1. **Configurable pricing:** Do not hardcode the `ModelPricing` table in Rust source. Instead:
   - Ship a default `pricing.toml` with Clido (e.g. in `clido-providers/data/pricing.toml` or embedded in the binary) containing per-model input/output/cache_creation/cache_read prices for major models (Claude Opus, Sonnet, Haiku, GPT-4o, etc.).
   - At startup, load pricing: first look for `{config_dir}/clido/pricing.toml` (user override); if absent, use the shipped default. Document in user docs how to override (e.g. copy the default file and edit). This allows users to update prices when providers change them without recompiling.
   - Define a `ModelPricing` struct and a loader `fn load_pricing(config_dir: Option<&Path>) -> ModelPricing` that merges or falls back to built-in defaults when the file is missing or a model is not listed.
2. **Staleness warning:** At startup, if the active `pricing.toml` (whether the default or user override) was last modified more than 90 days ago, emit a startup warning: `"Warning: your pricing.toml is {N} days old. Provider prices may have changed. Run 'clido update-pricing' or edit {path} to update."`. The `clido doctor` command (Phase 8.4) also surfaces this as a non-fatal warning. Record the file's mtime when loading; compare to `SystemTime::now()`.
3. **`clido update-pricing` command (V2+):** For V1, pricing updates are manual (edit the TOML). From V2 onward, optionally add an `clido update-pricing` subcommand that fetches a canonical pricing file from a hosted URL (e.g. `https://raw.githubusercontent.com/your-org/clido/main/data/pricing.toml`) and writes it to `{config_dir}/clido/pricing.toml`. This command is offline-safe: it must fail gracefully with a clear message if the network is unavailable, never corrupting the existing file. Implementation: download to a temp file, validate TOML parse, then rename atomically.
4. **Missing model fallback:** If a model is requested that is not listed in the pricing table, log a `warn!` and fall back to a configurable `default_pricing` entry rather than erroring out. This prevents cost tracking failure from blocking the agent.
5. In `clido-agent/src/loop.rs`, add:
   - `cumulative_cost_usd: f64`
   - After each model call: estimate cost from usage tokens × per-token price (from the loaded `ModelPricing`).
6. After each turn, check: `if cumulative_cost_usd > config.max_budget_usd → return error`.
7. Print cost summary in result message: `"Total cost: $0.0123 USD"`.
8. Store `total_cost_usd` in `SessionLine::Result`.
9. Test: load pricing.toml with a 91-day-old mtime (use `tempfile` + `filetime` in the test) → verify staleness warning is emitted. Test: request a model not in the pricing table → verify warning is logged and default price is used (no panic).

---

### Milestone 4.5 — Plan Mode

**Dependency:** 4.3 complete.

**Reference:** Plan mode UX, ExitPlanMode tool behavior, and session start notice are defined in [cli-interface-specification.md](cli-interface-specification.md) §5 and §7 (D6).

#### 4.5.1 Implement plan mode flag

1. `--permission-mode plan` → only read-only tools allowed.
2. In `PermissionChecker`: `PlanOnly` restricts to `Read`, `Glob`, `Grep` (no Bash, no Write, no Edit).
3. Agent still loops; model can still request state-changing tools, but they'll be denied with an inline message per spec (e.g. `[plan mode] Edit blocked — ExitPlanMode to allow`).
4. **ExitPlanMode tool:** Register a tool `ExitPlanMode` (no parameters). When the model calls it, switch the session to agent mode (state-changing tools allowed). Display the inline announcement per spec (rich: "⚡ Switching to agent mode — state-changing tools now available"; ASCII: `[mode] Switching to agent mode - state-changing tools now available`). Session start in plan mode must mention: "Clido is running in plan mode. State-changing tools are disabled. The agent can switch to agent mode using ExitPlanMode."
5. Test: run in plan mode; verify Write/Edit/Bash are blocked; call ExitPlanMode; verify state-changing tools then succeed and announcement was shown.

---

### Milestone 4.6 — Parallel Tool Execution with Bounded Concurrency

**Dependency:** 4.1, 3.1 complete.

#### 4.6.1 Detect read-only tools

1. The `is_read_only()` method is already defined on `Tool` trait (Phase 2.2.1).
2. Ensure all tools return the correct value:
   - `Read`, `Glob`, `Grep` → `true`.
   - `Write`, `Edit`, `Bash` → `false`.

#### 4.6.2 Parallelize read-only tool calls and mixed batch ordering

1. In `clido-agent/src/executor.rs`, when the model returns multiple `ToolUse` blocks in one response, use this **explicit algorithm**:
   - **(a)** Execute all read-only tools concurrently (under the semaphore from 4.6.3), and collect their results.
   - **(b)** Execute state-changing tools one at a time, in the order they appear in the model response; collect their results.
   - **(c)** Return all results to the model in the **original tool-call order** (so the model sees results aligned with its request order).
   - If the batch is all read-only: only step (a) applies. If all state-changing: only step (b) applies. Mixed: do (a) then (b), then merge results into the original order.
2. Collect results preserving original tool call ordering in all cases.
3. Test: model requests `[Read, Read, Read]` → verify all three execute concurrently.
4. Test: model requests `[Read, Edit, Read]` → verify both Reads run first (concurrently), then Edit runs; results returned in order Read, Edit, Read.

#### 4.6.3 Add semaphore-based concurrency limit

1. In `clido-agent/src/executor.rs`, add a `Semaphore` to `ToolExecutor`:

   ```rust
   pub struct ToolExecutor {
       tools: Arc<ToolRegistry>,
       semaphore: Arc<Semaphore>,  // default: 10 permits
   }
   ```

2. Before every tool execution (parallel or sequential): acquire a semaphore permit.
3. Release permit after tool completes (permit dropped on scope exit).
4. Configure max concurrency in `config.toml`: `[tools] max_concurrent = 10`.
5. Test: spawn 20 simultaneous Read calls → verify at most 10 execute concurrently (measure with a slow mock tool).
6. Verify: no deadlock when all 10 permits are held and a sequential (state-changing) tool is queued.

---

### Milestone 4.7 — Subagent Architecture

**Dependency:** 4.6 complete, 6.3 (Arc wrapping) can be done early.

**Rationale:** Claude Code uses subagents heavily (traces show `Agent` tool with 39 calls, second-highest after Read/Bash/Edit). Subagents allow isolated parallel reasoning tasks (e.g., parallel file analysis, running tests while editing).

#### 4.7.1 Define SubAgent types

1. In `clido-agent/src/subagent.rs`:

   ```rust
   pub struct SubAgentConfig {
       pub prompt: String,
       pub description: Option<String>,    // human-readable label
       pub subagent_type: SubAgentType,
       pub max_turns: u32,
   }

   pub enum SubAgentType {
       GeneralPurpose,
       ReadOnly,      // plan mode — only read tools
       Custom(String),
   }

   pub struct SubAgentHandle {
       pub id: String,
       pub task: tokio::task::JoinHandle<Result<String>>,
   }
   ```

2. Derive `Serialize`, `Deserialize`, `Debug`, `Clone` on config types.

#### 4.7.2 Implement SubAgentManager

1. In `clido-agent/src/subagent.rs`:

   ```rust
   pub struct SubAgentManager {
       provider: Arc<dyn ModelProvider>,
       tools: Arc<ToolRegistry>,
       active: HashMap<String, SubAgentHandle>,
       /// Shared with parent: cumulative cost in USD. All subagent spending is charged here.
       cost_accumulator: Arc<CostAccumulator>,
       max_budget_usd: f64,
   }
   ```

   Use a thread-safe cost accumulator (e.g. `Arc<AtomicU64>` storing cents, or `Arc<RwLock<f64>>` for USD) shared between the parent `AgentLoop` and the `SubAgentManager`. The parent's `AgentConfig.max_budget_usd` is passed into the manager.

2. `spawn()`:
   - **Budget check:** Before creating a subagent, check that `cost_accumulator.current() + subagent_cost_estimate` does not exceed `max_budget_usd`. The estimate can be conservative (e.g. assume one turn at current model price). If the check fails, return an error (e.g. `ClidoError::BudgetExceeded`) and do not spawn.
   - Create a new `AgentLoop` with shared `Arc<dyn ModelProvider>`, `Arc<ToolRegistry>`, and the same `cost_accumulator`. The subagent's cost tracking (Phase 4.4) must write back to this shared accumulator so that every dollar spent by a subagent counts against the parent's budget.
   - Apply `subagent_type` restrictions (ReadOnly → PlanOnly permission mode).
   - Spawn with `tokio::task::spawn(async move { agent.run(&config.prompt).await })`.
   - Store handle in `active`.
   - Return subagent id.
3. `wait()`: await the join handle for a given id, return result.
4. `wait_all()`: `futures::future::join_all` over all active handles.
5. **Test:** Parent has `max_budget_usd = 0.10`. Spawn two subagents that each spend ~$0.06; then attempt to spawn a third. Verify the third spawn is refused (budget would be exceeded) and the error is returned to the caller.

#### 4.7.3 Implement `AgentTool` (spawns subagents from within the loop)

1. Add `AgentTool` to `clido-tools/src/`:
   - Parameters: `prompt` (string, required), `description` (string, optional), `subagent_type` (string, optional).
   - `is_read_only()` → `false` (subagents may write).
   - `execute()`: calls `SubAgentManager::spawn()` and immediately waits (synchronous from the outer agent's perspective).
   - Returns subagent's final text output as `ToolOutput`.
2. Register `AgentTool` in the default tool registry.
3. Test: outer agent spawns a subagent via `AgentTool`; subagent reads a file; outer agent receives file summary.

#### 4.7.4 Subagent session recording

1. Subagents write their own JSONL session file under `{session_id}/subagents/{subagent_id}.jsonl`.
2. Parent session file records `AgentTool` use and result as normal tool_use/tool_result blocks.
3. Test: verify subagent session file exists after subagent completes.

---

### Milestone 4.8 — Task Graph / Planner (Advanced, Optional)

**Dependency:** 4.7 complete.

**Design note:** The reactive agent loop (Phases 2–4.7) is the primary execution model and handles most tasks well. The planner is an *optional overlay* for tasks where the user or agent benefits from a structured upfront plan. It does not replace the reactive loop — it pre-structures it. The planner call is itself non-deterministic (it is a model call), so the "determinism" is only in the execution layer once a valid graph is produced.

**Trade-offs:**
- Adds one extra model call per task (latency + cost).
- Fails gracefully: if the planner produces an invalid graph, fall back to the reactive loop.
- Best suited for large, well-defined tasks (e.g., "refactor all usages of X across 50 files").
- Not recommended for open-ended exploration where each step depends on findings.

#### 4.8.1 Define task graph types

1. In `clido-planner/src/task_graph.rs`:

   ```rust
   pub struct Task {
       pub id: String,
       pub tool: String,
       pub input: serde_json::Value,
       pub depends_on: Vec<String>,
       pub description: Option<String>,
   }

   pub struct TaskGraph {
       pub tasks: Vec<Task>,
   }

   impl TaskGraph {
       /// Topological sort; returns tasks in execution order with parallelism groups.
       pub fn execution_order(&self) -> Result<Vec<Vec<Task>>>;
       /// Validate: no cycles, all depends_on ids exist, all tool names valid.
       pub fn validate(&self, registry: &ToolRegistry) -> Result<()>;
   }
   ```

2. Tests:
   - Valid linear graph: verify topological order.
   - Valid parallel graph (two tasks with no dependency): verify both in same group.
   - Cyclic graph: verify `validate()` returns error.
   - Unknown tool name: verify `validate()` returns error.

#### 4.8.2 Define Planner trait

1. In `clido-planner/src/planner.rs`:

   ```rust
   #[async_trait::async_trait]
   pub trait Planner: Send + Sync {
       async fn plan(
           &self,
           prompt: &str,
           context: &[Message],
           registry: &ToolRegistry,
       ) -> Result<TaskGraph>;
   }
   ```

2. Implement `ModelPlanner`:
   - Builds a system prompt instructing the model to output a JSON `TaskGraph`.
   - Calls `provider.complete()` with a single user message containing the task prompt.
   - Parses response as JSON; deserializes into `TaskGraph`.
   - Validates the graph; on failure returns `ClidoError::Planner`.
   - System prompt example:

     ```
     You are a task planner. Given a user request, output a JSON task graph.
     Each task has: id, tool, input (matching the tool's schema), depends_on (list of ids).
     Available tools: {tool_list}
     Output only valid JSON. No markdown, no explanation.
     ```

3. Test: mock provider returning a known JSON graph → verify correct `TaskGraph` parsed.

#### 4.8.3 Implement TaskExecutor

1. In `clido-planner/src/executor.rs`:

   ```rust
   pub struct TaskExecutor {
       tools: Arc<ToolRegistry>,
       semaphore: Arc<Semaphore>,
   }

   impl TaskExecutor {
       pub async fn execute(
           &self,
           graph: TaskGraph,
           results: &mut HashMap<String, ToolOutput>,
       ) -> Result<ExecutionSummary>;
   }
   ```

2. `execute()` logic:
   - Call `graph.execution_order()` to get groups.
   - For each group (tasks with satisfied dependencies): execute all tasks in the group concurrently (respecting semaphore).
   - Store results in `results` map keyed by task id.
   - If a task fails (`is_error: true`): record failure, continue executing independent tasks; collect all errors in `ExecutionSummary`.
   - **Partial execution visibility:** When execution stops (due to failure or user interrupt), `ExecutionSummary` must report which node ids succeeded and which failed. Emit this to the user (and to the session/audit log) so the codebase state is interpretable (e.g. "Nodes A, B, C completed; node D failed.").

#### 4.8.3a Checkpoint before planner execution and resume-plan

1. **Checkpoint:** Before running `TaskExecutor::execute()` (same mechanism as Milestone 5.6): if cwd is a git repository, record a baseline (e.g. `git status --porcelain`, `git diff --stat` or a stash). If execution fails partway, the user can restore from this baseline to undo partial edits.
2. **On graph execution failure:** Report which nodes succeeded and which failed (see 4.8.3). Do not leave the user without a clear picture of what was applied.
3. **Resume-plan:** Provide an `clido resume-plan {session_id}` (or equivalent) mechanism: load the last executed plan and its `ExecutionSummary` from the session; allow the user to re-run from the next unexecuted node (or re-run from a chosen node). This requires persisting the plan and per-node results in the session so that a subsequent run can continue rather than re-execute from scratch. Document that resuming may require manual fix of the codebase if files were changed between the failed run and the resume.

#### 4.8.4 Implement plan-refine loop

1. After `TaskExecutor::execute()` completes, feed `ExecutionSummary` back to the model:
   - Message: `"I executed the following plan. Here are the results. Do you need to revise the plan or are you done? Reply with 'DONE: {final answer}' or an updated task graph JSON."`.
   - If model replies `DONE: ...`: extract final answer and return.
   - If model replies with a new task graph: validate, execute, repeat (up to `max_refinement_iterations`, default 3).
2. This is the "plan-refine loop" — note it is structurally equivalent to the reactive loop but with graph structure per iteration.
3. Test: mock a failing first execution; verify model refines the plan and second execution succeeds.

#### 4.8.5 Wire planner into CLI

1. Add `--planner` flag to `clido-cli`.
2. If `--planner` flag present or `config.use_planner = true`:
   - Run `ModelPlanner::plan()` first.
   - Display the plan to user (see Milestone 8.6).
   - Ask confirmation: `"Execute this plan? [Y/n]"`.
   - Run `TaskExecutor::execute()`.
   - Feed results to reflection step.
3. On planner failure: log warning, fall back to reactive loop automatically.
4. Test: planner mode with mock provider → verify plan displayed, confirmed, executed.

---

## Phase 5 — Reliability Improvements

**Goal:** Robust error handling, retry logic, session recovery, graceful shutdown, persistent memory.

---

### Milestone 5.1 — Robust Error Handling

**Dependency:** Phase 4 complete.

#### 5.1.0 Shared retry and backoff (all providers)

1. In `clido-providers/src/retry.rs`:
   - Implement a shared middleware (or helper) used by **every** provider (Anthropic, OpenAI, OpenRouter, Alibaba, and any future HTTP-based provider).
   - On HTTP 429 (rate limit): parse `Retry-After` header when present; otherwise use exponential backoff: 1s, 2s, 4s, 8s. Max 3 retries.
   - On HTTP 5xx: retry up to 3 times with the same backoff.
   - Use `tokio::time::sleep` between retries. Log each retry at `warn!` level.
   - On HTTP 4xx (except 429): do not retry, return `ClidoError::Provider`.
   - Expose something like `async fn with_retry<F, Fut>(f: F) -> Result<...>` so each provider's `complete()` can wrap its HTTP call.
2. **Anthropic** (5.1.1): Use the shared retry in `clido-providers/src/anthropic.rs` for every `complete()` request.
3. **OpenAI / OpenRouter / Alibaba** (Phase 4.1): When implementing those providers, use the same `retry.rs` middleware for their HTTP calls. Do not implement retry only for Anthropic; all providers must use the shared logic.
4. Test: mock server returning 429 → verify retry behavior for at least one provider; test that a second provider (e.g. OpenAI) also retries when its mock returns 429.

#### 5.1.1 Anthropic provider uses shared retry

1. In `clido-providers/src/anthropic.rs`, wrap every `complete()` HTTP request with the shared `retry::with_retry` (or equivalent) from 5.1.0.
2. Test: mock Anthropic server returning 429 → verify retry behavior (as in 5.1.0).

#### 5.1.2 Handle tool execution failures gracefully

1. From trace evidence: failed tool results (`is_error: true`) are passed to the model as normal `tool_result` messages; the model decides how to recover.
2. Ensure `ToolOutput` with `is_error: true` is always returned (never panic).
3. Add catch-all in `execute()`: `tokio::task::spawn(async { tool.execute(input).await }).await.unwrap_or_else(|e| ToolOutput { content: format!("Internal error: {e}"), is_error: true })`.
4. Test: tool that panics → verify panic is caught and returned as is_error.

#### 5.1.3 Validate tool input schemas before execution

1. In `clido-tools/src/registry.rs`:
   - Before calling `tool.execute(input)`, validate `input` against the tool's `schema()` using `jsonschema` crate.
   - If validation fails: return `ToolOutput { is_error: true, content: "InputValidationError: ..." }` without executing.
2. Test: call Grep with unknown parameter → verify InputValidationError (matching observed format).

#### 5.1.4 Handle malformed JSON in model tool calls

1. When parsing `ToolUse.input`: use `serde_json::from_str` with fallback.
2. If JSON is malformed (streaming partial): wait for `ToolUseEnd`, retry parse.
3. If still malformed after assembly: return `is_error: true`, `"Malformed tool input JSON"`.
4. Test: inject a partial JSON fragment; verify it's assembled correctly.

---

### Milestone 5.2 — Session Recovery

**Dependency:** 3.3, 5.1 complete.

#### 5.2.1 Checkpoint writes

1. Write each session line immediately after it's generated (not buffered).
2. On crash/kill: session file contains all turns up to crash point.
3. `--resume` reconstructs history from file and continues.
4. Test: kill the process mid-session; resume with `--resume`; verify continuation.

#### 5.2.2 Implement session fork

1. Add `SessionStorage::fork(session_id: &str) -> Result<String>`:
   - Copies session JSONL to a new session_id file.
   - Returns new session_id.
2. Expose via `clido fork {session_id}` subcommand.
3. Test: fork a session; run both; verify they diverge independently.

#### 5.2.3 Validate file state on resume

1. When `--resume {session_id}` is used, before reconstructing the loop and continuing:
   - Load from the session JSONL the list of every file that was successfully edited in the previous run, together with the mtime (or content hash) recorded at the time of that edit (see 5.6.2 for how these are stored).
   - For each such file: read current mtime (or compute current content hash); compare to the stored value.
   - If any file has changed: warn the user and offer to abort resume, or continue anyway (resuming may then apply edits to outdated content and produce incorrect diffs or corruption). In non-interactive mode, exit with an error unless the user has passed a flag such as `--resume-ignore-stale`.
2. Implementation details (prompt text, flag name, and tests) are in Milestone 5.6.4.
3. Test: complete a session with one Edit; change that file on disk; run `clido --resume {id}`; verify validation runs and user is prompted or error is returned.

---

### Milestone 5.3 — Graceful Shutdown

**Dependency:** Phase 4 complete.

#### 5.3.1 Handle Ctrl-C

1. Install `tokio::signal::ctrl_c()` handler in `clido-cli/src/main.rs`.
2. On Ctrl-C:
   - If tool is currently executing: let it finish or timeout after 2s.
   - Print `"\nInterrupted. Session saved to {session_id}."`.
   - Flush session file.
   - Exit cleanly.
3. Test: interrupt during Bash execution → verify session is flushed.

---

### Milestone 5.4 — Integration Test Suite

**Dependency:** 5.1–5.3 complete.

#### 5.4.1 Build integration test harness

1. In `tests/integration/`:
   - Create a mock model provider that returns pre-scripted responses.
   - Fixture repository: `tests/fixtures/sample-project/` with known files.
2. Test scenarios:
   - Single-turn: model returns text → verify output.
   - Tool use: model requests Read → verify file returned → model returns text.
   - Edit: model requests Edit → verify file changed on disk.
   - Error recovery: model requests bad Edit → verify is_error → model requests new Edit.
   - Max turns: model keeps requesting tools → verify loop stops at max_turns.
   - Cost limit: inject high usage → verify cost limit stops loop.
   - Semaphore: 20 concurrent Read calls with semaphore=5 → verify bounded.
   - Subagent: outer agent spawns subagent → subagent reads file → outer receives result.

#### 5.4.2 Minimum coverage targets (enforced in CI)

Coverage is measured with `cargo-tarpaulin --all-features`. CI fails if overall coverage drops below the floor for that release. Target floors are intentionally modest for V1 to avoid blocking shipping, and raise each release as the codebase matures.

| Crate | V1 floor | V2 floor | V3 floor | V4 floor |
|---|---|---|---|---|
| `clido-agent` (agent loop, executor) | **70 %** | 75 % | 80 % | 85 % |
| `clido-providers` (all providers) | **75 %** | 80 % | 85 % | 85 % |
| `clido-context` (context engine, compaction) | **70 %** | 75 % | 80 % | 80 % |
| `clido-tools` (all built-in tools) | **80 %** | 85 % | 85 % | 85 % |
| `clido-session` (JSONL persistence) | **75 %** | 80 % | 80 % | 80 % |
| `clido-memory` *(V3+)* | — | — | **70 %** | 80 % |
| Workspace overall | **70 %** | 75 %  | 78 % | 82 % |

**V1 critical-path tests (must pass before V1 release, regardless of coverage):**
- Agent loop: single-turn, multi-turn with tool use, max_turns halt, cost-limit halt.
- Anthropic provider: request serialization, streaming parse, error mapping, retry on 429.
- Permission gate: `AcceptAll`, `PlanOnly`, and `Default` with `AskUser` (serialized via `Mutex`).
- Session: write, read-back, session resume.
- Config: profile resolution, missing API key error, unsupported provider error.

---

### Milestone 5.5 — Memory System

**Dependency:** 5.1 complete, 3.3 complete.

**Rationale:** Session history covers a single run. Memory persists knowledge across sessions: repo structure, user preferences, past findings, recurring patterns.

The memory system uses a **hybrid retrieval strategy**: FTS5 full-text search for exact keyword matches combined with vector similarity search (`sqlite-vec`) for semantic matches. Embeddings are generated locally via `fastembed-rs` (ONNX inference, no API calls, no internet required). This gives Cursor-level retrieval quality while remaining fully local and offline.

#### 5.5.1 Define memory types

1. In `clido-memory/src/lib.rs`:

   ```rust
   pub enum MemoryType {
       ShortTerm,  // in-session working notes
       LongTerm,   // cross-session persistent
   }

   pub struct MemoryEntry {
       pub id: String,
       pub content: String,
       pub tags: Vec<String>,
       pub created_at: DateTime<Utc>,
       pub session_id: Option<String>,
       pub embedding: Option<Vec<f32>>,  // 384-dim vector (bge-small-en-v1.5)
   }

   pub struct MemorySearchResult {
       pub entry: MemoryEntry,
       pub score: f32,       // combined relevance score (0.0–1.0)
       pub match_type: MatchType,
   }

   pub enum MatchType {
       Keyword,   // FTS5 hit
       Semantic,  // vector similarity hit
       Hybrid,    // both
   }
   ```

2. Define `MemoryStore` trait:

   ```rust
   #[async_trait::async_trait]
   pub trait MemoryStore: Send + Sync {
       async fn save(&self, entry: &MemoryEntry) -> Result<()>;
       /// Hybrid search: FTS5 + vector similarity, merged by Reciprocal Rank Fusion.
       async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>>;
       async fn search_keyword(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>>;
       async fn search_semantic(&self, embedding: &[f32], limit: usize) -> Result<Vec<MemorySearchResult>>;
       async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryEntry>>;
       async fn delete(&self, id: &str) -> Result<()>;
   }
   ```

#### 5.5.2 Implement short-term memory (in-session)

1. In `clido-memory/src/short_term.rs`:
   - `InMemoryStore`: `HashMap<String, MemoryEntry>` guarded by `tokio::sync::RwLock`.
   - `search()`: substring match on `content` (no embedding needed for in-session speed).
   - Used for: noting intermediate findings within a session that should be referenced later.
2. Test: save 5 entries; search by keyword; verify correct entries returned.

#### 5.5.3 Implement local embedding engine

1. In `clido-memory/src/embeddings.rs`:
   - Use `fastembed` crate (ONNX-based, local inference, no API key, no internet).
   - Model: `EmbeddingModel::BGESmallENV15` — 384-dim, ~25MB model file, fast CPU inference.
   - Model file is downloaded once on first use to `{data_dir}/models/` and cached locally.
   - Wrap in `EmbeddingEngine` struct:

     ```rust
     pub struct EmbeddingEngine {
         model: TextEmbedding,  // fastembed::TextEmbedding
     }

     impl EmbeddingEngine {
         pub fn new(data_dir: &Path) -> Result<Self>;
         /// Generate a single embedding. Runs in a spawn_blocking task.
         pub async fn embed(&self, text: &str) -> Result<Vec<f32>>;
         /// Batch embed for efficiency. Runs in a spawn_blocking task.
         pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
     }
     ```

   - `embed()` calls `tokio::task::spawn_blocking` to avoid blocking the async runtime.
2. Test: embed two semantically similar strings ("async Rust", "concurrent Tokio code") → cosine similarity > 0.8. Embed two unrelated strings → similarity < 0.3.
3. Test: model file is created at expected path after first `EmbeddingEngine::new()`.

#### 5.5.4 Implement long-term memory (SQLite + FTS5 + sqlite-vec)

1. In `clido-memory/src/long_term.rs`:
   - Use `rusqlite` (sync, via `tokio::task::spawn_blocking`).
   - Load `sqlite-vec` extension on connection open: `conn.load_extension(sqlite_vec::sqlite3_vec_init, None)`.
   - Schema:

     ```sql
     -- Main store
     CREATE TABLE memories (
         id TEXT PRIMARY KEY,
         content TEXT NOT NULL,
         tags TEXT,              -- JSON array
         created_at TEXT NOT NULL,
         session_id TEXT
     );

     -- FTS5 index for keyword search
     CREATE VIRTUAL TABLE memories_fts USING fts5(
         content, tags,
         content=memories, content_rowid=rowid
     );

     -- Vector index for semantic search (384-dim bge-small-en-v1.5)
     CREATE VIRTUAL TABLE memories_vec USING vec0(
         rowid INTEGER PRIMARY KEY,
         embedding FLOAT[384]
     );
     ```

   - `save(entry)`:
     1. Insert into `memories`.
     2. Insert into `memories_fts` (trigger or explicit insert).
     3. If `entry.embedding` is `Some(v)`: insert into `memories_vec`.
   - `search_keyword(query, limit)`: FTS5 query, return ranked results.
   - `search_semantic(embedding, limit)`: `SELECT rowid, distance FROM memories_vec WHERE embedding MATCH ? ORDER BY distance LIMIT ?`, join with `memories`.
   - `search(query, limit)`: run both, merge with **Reciprocal Rank Fusion** (RRF):
     - RRF score: `Σ 1/(k + rank_i)` where `k = 60` (standard constant).
     - Return top `limit` entries sorted by combined score.

2. DB path: `{data_dir}/memory.db`.
3. Test: save 100 entries; FTS5 search for exact term → verify match. Semantic search for paraphrase → verify relevant result found even without shared keywords. Hybrid search → verify combined ranking is better than either alone.
4. Test: persistence across process restart.
5. Test: memories without embeddings are still found by keyword search (embedding is optional).

#### 5.5.5 Wire memory into agent loop

1. Add `memory: Arc<dyn MemoryStore>` and `embeddings: Arc<EmbeddingEngine>` to `AgentLoop`.
2. At session start: retrieve **relevant** memories by querying against the initial user prompt (not by recency):
   - Generate embedding of the user's first message via `EmbeddingEngine::embed()`.
   - Call `memory.search(user_prompt, 10)` — returns hybrid-ranked results.
   - Inject the top results as a system prompt section:

     ```
     ## Relevant Memory
     - {entry.content}
     ```

   - This ensures the 10 injected memories are semantically relevant to the current task, not just the most recent ones.

3. Implement `MemoryTool` in `clido-tools/`:
   - Parameters: `action` (`save` | `search` | `list`), `content` (for save), `query` (for search).
   - On `save`: generate embedding for `content`, populate `entry.embedding`, call `memory.save()`.
   - On `search`: call `memory.search(query, 10)` (hybrid).
   - Register in tool registry.
   - Model can explicitly save insights: `MemoryTool(action="save", content="This repo uses X pattern for Y")`.
4. Test: save a memory in session 1; start session 2 with a semantically related (but not identical) prompt; verify the memory surfaces in context.
5. Test: save a memory about topic A; start session 2 with a completely unrelated prompt about topic B; verify memory about A does NOT appear in context (relevance filtering works).

#### 5.5.6 Memory capacity, deduplication, and pruning

1. **Max entries and eviction:** Add a configurable `max_memory_entries` (default 10,000) to the long-term store. When inserting a new entry would exceed this limit, evict the least-recently-used entry (or the oldest by `created_at`) so the store size stays bounded. Config key e.g. `[memory] max_entries = 10000`.
2. **Deduplication on save:** Before inserting a new memory, run a semantic similarity check: if an existing entry has cosine similarity to the new entry's embedding above a threshold (e.g. 0.95), treat the new save as a no-op (skip insert, or update the existing entry's `created_at` so it is not evicted soon). This avoids storing many near-duplicate insights across sessions.
3. **Per-session write cap:** To prevent runaway `MemoryTool` usage from flooding the store (a misguided model saving every line of output), enforce a configurable `max_memory_writes_per_session` (default: 50). Config key: `[memory] max_writes_per_session = 50`. When the cap is reached mid-session, `MemoryTool` returns `is_error: true` with the message: `"Memory write limit reached for this session ({n} of {max}). Clido will not save more memories this session. Existing memories are still readable."`. The cap resets at the start of each new session. Rationale: without a write cap, a model that repeatedly calls `MemoryTool` in a long session can fill `max_memory_entries` with low-quality entries, evicting valuable older memories. The cap forces selective saving.
4. **Manual prune command:** Implement `clido memory prune` (or `clido memory cleanup`): optionally accept `--older-than-days N` or `--keep N`; remove entries that do not meet the keep criteria. Without arguments, prune to `max_memory_entries` by evicting oldest first. This gives users control when the automatic eviction is not enough.
5. Test: insert 10,001 entries with max_entries=10,000 → verify size stays at 10,000. Save the same content twice (or two very similar contents) → verify dedup prevents duplicate. Run `clido memory prune --keep 100` → verify only 100 entries remain.
6. Test: call `MemoryTool` 51 times in one session (max_writes_per_session=50) → verify 51st call returns `is_error: true` with cap message; verify 50 entries were written, not 51.
7. Document in user docs: how to inspect memory size, how to adjust `max_memory_entries` and `max_writes_per_session`, and when to run manual prune.

---

### Milestone 5.6 — Edit Safety and Partial Write Detection

**Dependency:** 5.1 complete, 5.2 complete, 3.3 complete.

**Rationale:** If Clido makes multiple `Edit` calls in a session and edit 1–3 succeed but edit 4 fails catastrophically (crash, SIGKILL), the codebase is left in a partial state. Graceful edit failures return `is_error: true` and the model can recover; unclean exits do not. This milestone adds detection and user visibility so operators can recover from partial edits.

#### 5.6.1 Session start: baseline snapshot

1. At session start (before the first turn), if the current working directory is inside a git repository:
   - Run `git status --porcelain` and `git diff --stat` (or equivalent via `git2` crate); store the output and the list of modified/untracked paths.
   - Store this baseline in the session metadata (e.g. first line of the JSONL or a separate `session_meta.json`).
2. If not a git repo: record a list of file paths that will be considered "touched" as edits occur (no baseline diff possible; only track which files were edited in-session).
3. Test: start session in a git repo with one modified file; verify baseline is recorded.

#### 5.6.2 Track edited files per session

1. In the session writer (Phase 3.3), for every `Edit` tool call that completes successfully, append the edited file path and its post-edit mtime (or content hash) to a "touched files" list in session metadata or in each relevant `SessionLine`.
2. On unclean exit (no final `SessionLine::Result`), this list is still available for the next run.

#### 5.6.3 Unclean exit: warn user

1. On process exit, if the session did not complete normally (no final result line, or process received SIGINT/SIGTERM):
   - If a baseline was recorded: run `git status --porcelain` again; diff against baseline to list files that were modified during the session.
   - Print to stderr: `"Session ended without completing. The following files were modified during this session: {list}. Review changes with git diff or resume with clido --resume {session_id}."`
2. Test: start session, perform one Edit, kill process (SIGTERM); verify warning lists the edited file.

#### 5.6.4 Session resume: detect stale edited files

1. When resuming a session (Phase 5.2), load the list of files that were edited in the previous run (from session JSONL / metadata).
2. For each such file: read current mtime (or hash); compare to the value stored at the time of the last successful edit in that session.
3. If any file has changed (mtime or hash differs): before continuing, prompt the user (or in non-interactive mode, emit a warning and exit with an error unless `--resume-ignore-stale` is set): `"The following files were modified since the last session run: {list}. Resuming may apply edits to outdated content. Abort (a), continue anyway (c), or exit (e)."`
4. Test: run session with one Edit; manually change the edited file; run `clido --resume {id}`; verify warning and choice are offered.

---

## Phase 6 — Performance Optimization

**Goal:** Fast startup, low memory, efficient context building, file read caching.

---

### Milestone 6.1 — Startup Performance

**Dependency:** Phase 5 complete.

#### 6.1.1 Profile startup time

1. Add `tracing::span!` around startup steps: config load, provider init, tool registry build.
2. Use `cargo flamegraph` or `samply` to profile a simple `clido -p "hello"` run.
3. Target: `< 200ms` to first API request.

#### 6.1.2 Lazy provider initialization

1. Wrap provider in `tokio::sync::OnceCell`; initialize on first use.
2. Do not perform any I/O in `ProviderFactory::build()` — only construct structs.
3. Move connection test (if any) to an optional `clido doctor` command.

#### 6.1.3 Optimize JSON serialization

1. Replace `.to_string()` + `from_str()` round-trips with direct serialization where possible.
2. Use `serde_json::to_writer` instead of `to_string` when writing to files.
3. Profile: `cargo bench` for serialization hot paths.

---

### Milestone 6.2 — Context Efficiency

**Dependency:** 4.2 complete.

#### 6.2.1 Implement smart file content truncation

1. For `Read` tool results exceeding 50k characters: truncate with a note.
   - Exact truncation: show first N lines, add `"\n[... truncated {remaining} lines ...]"`.
   - Allow model to use `offset`/`limit` params to read specific sections.
2. Test: read a large file; verify truncation is applied and noted.

#### 6.2.2 Deduplicate repeated file reads in context

1. Track which file paths have been Read in this session.
2. On a second Read of the same path with same content: return `"[File content unchanged from previous read]"` if unchanged.
3. This reduces context bloat on repeated reads.

---

### Milestone 6.3 — Concurrent Provider Requests

**Dependency:** 6.1 complete.

#### 6.3.1 Ensure AgentLoop is cheaply constructable for subagents

1. `AgentLoop` should hold only `Arc` references so construction is O(1).
2. Wrap shared state (tool registry, provider, memory store) in `Arc`.
3. Test: two concurrent `AgentLoop::run()` calls on the same provider → no data races.

---

### Milestone 6.4 — File Read LRU Cache

**Dependency:** 6.1 complete.

**Rationale:** Agents frequently re-read the same files within a session (trace pairs: Read→Read→Read = 537 occurrences). Caching avoids redundant disk I/O and can serve unchanged content instantly.

#### 6.4.1 Implement in-memory LRU cache for file reads

1. In `clido-tools/src/cache.rs`:
   - Use `lru` crate: `LruCache<PathBuf, CacheEntry>` — path → `CacheEntry`.
   - `CacheEntry` stores `content: String`, `mtime: SystemTime`, and `content_hash: u64` (xxHash or Blake3 of the content, chosen for speed not security).
   - Cache capacity: configurable, default 50 entries.
   - **Invalidation strategy — mtime + content hash:**
     - On `get(path)`: read current file mtime via `fs::metadata()`.
       - If `current_mtime == cached_mtime`: return cached content (fast path, no disk read).
       - If `current_mtime != cached_mtime` (file timestamp changed): read file from disk, compute `new_hash`. If `new_hash == cached_hash`, update the cached mtime but keep the cached content (file was touched but not modified — e.g. `touch`, `rsync --times`). If `new_hash != cached_hash`, evict and re-cache with new content, mtime, and hash.
     - **Rationale for dual check:** mtime alone fails in two edge cases: (a) files modified within the same filesystem timestamp granularity (e.g. FAT32 with 2-second resolution) — two edits within 1 second appear as same mtime; (b) files restored to the same content (e.g. `git checkout`) keep a new mtime but are semantically identical — no need to evict. The hash provides a second-opinion that avoids both false positives and false negatives.
   - `insert(path, content, mtime)` → compute and store hash, store in LRU.
2. Wrap in `Arc<Mutex<FileCache>>` for shared use.

#### 6.4.2 Wire cache into ReadTool

1. `ReadTool` gets a `cache: Arc<Mutex<FileCache>>` reference (passed at construction).
2. Before reading from disk: check cache.
   - Cache hit and file unmodified: return cached content (mark in response: no additional note needed, this is transparent).
   - Cache miss or file modified: read from disk, insert into cache.
3. Test: read the same file twice; verify second read is served from cache (mock disk to confirm no second syscall).
4. Test: modify file between reads; verify cache is invalidated and fresh content returned.

#### 6.4.3 Disable cache for large files

1. Files over 1 MB: do not cache (to avoid excessive memory use).
2. Make threshold configurable: `[tools] cache_max_file_bytes = 1048576`.

---

## Phase 7 — Security and Sandboxing

**Goal:** Safe Bash execution, path traversal prevention, permission auditing.

---

### Milestone 7.1 — Bash Sandboxing

**Dependency:** Phase 5 complete.

#### 7.1.1 Restrict Bash environment

1. Strip sensitive env vars before spawning subprocesses: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `AWS_*`, `GITHUB_TOKEN`, etc.
2. Allow list of env vars that pass through (configurable).
3. Test: `echo $ANTHROPIC_API_KEY` in Bash → verify empty string returned.

#### 7.1.2 Platform sandboxing (macOS)

1. **Current approach (best-effort):** On macOS, wrap Bash commands in `sandbox-exec` with a restrictive profile when `--sandbox` is set:
   - Allow: read from cwd subtree, write to cwd subtree, exec `/bin/sh`, `/usr/bin/*`, `/usr/local/bin/*`.
   - Deny: network access, write outside cwd, access to `~/.config`, `~/.ssh`, `~/.gnupg`.
   - **Deprecation note:** `sandbox-exec` is deprecated as of macOS 14 Sonoma and may be removed in a future release. Users on current macOS may see deprecation warnings. Document in user-facing docs that `--sandbox` on macOS is best-effort on modern versions and that a hardened solution is a V2+ follow-on.
2. **Alternative paths (for a future hardened solution):**
   - **App Sandbox:** Use Apple's App Sandbox entitlements (requires code signing and possibly notarization). Suitable if Clido is distributed as a signed app bundle.
   - **Process-level containment:** Spawn Bash inside a helper that uses `sandbox-exec` only when available, or a lightweight container mechanism (e.g. `darwin-containers` or similar) if the ecosystem matures.
   - Until then: `--sandbox` on macOS remains best-effort; Linux seccomp/Docker sandbox (7.1.3) is the primary hardened path.
3. Gate behind `--sandbox` CLI flag (opt-in initially).
4. Test: command attempting network access inside sandbox → verify denied (when sandbox-exec is present).

#### 7.1.3 Platform sandboxing (Linux)

1. On Linux: use `seccomp-bpf` via the `seccomp` crate to restrict syscalls in child process.
2. Alternatively: spawn inside a minimal Docker container if `docker` is available and `--docker-sandbox` flag is set.
3. Test: verify sandboxed process cannot write outside cwd.

#### 7.1.4 Path traversal prevention for file tools

1. In `ReadTool`, `WriteTool`, `EditTool`:
   - Canonicalize path.
   - Optionally enforce a "root" directory (cwd by default).
   - If path escapes root: return `is_error: true`, `"Access denied: path outside working directory."`.
2. Allow user to configure trusted paths in `config.toml`.
3. Test: `Read { file_path: "/etc/passwd" }` → verify denied when cwd-restriction active.

---

### Milestone 7.2 — Secret Detection

**Dependency:** 7.1 complete.

#### 7.2.1 Scan Write/Edit content for secrets

1. Before writing content to disk: scan for patterns matching API keys, tokens, private keys:
   - Regex patterns: `sk-[A-Za-z0-9]{48}`, `ghp_[A-Za-z0-9]{36}`, `-----BEGIN (RSA |EC )?PRIVATE KEY-----`, etc.
2. If match found: warn user; require confirmation before writing.
3. Gate: only active if `[security] scan_writes = true` in config.
4. Test: write a fixture containing a fake API key → verify warning.

---

### Milestone 7.3 — Audit Logging

**Dependency:** 7.1 complete.

#### 7.3.1 Implement tool audit log

1. Every tool call (name, input summary, result summary, is_error, duration_ms) → append to `{data_dir}/audit.jsonl`.
2. This is separate from the session file; the audit log is append-only and never compacted.
3. Expose via `clido audit` subcommand: tail or search the audit log.

---

## Phase 8 — Developer Experience

**Goal:** Polished CLI, hooks system, MCP compatibility, live plan display, repo indexing, documentation.

**CLI contract:** Canonical CLI behavior (commands, flags, output formats, exit codes, and UX) is defined in [cli-interface-specification.md](cli-interface-specification.md). Implementations in this phase must align with that spec.

---

### Milestone 8.1 — Hooks System

**Dependency:** Phase 5 complete.

#### 8.1.1 Define hook interface

1. Hooks fire at: `PreToolUse`, `PostToolUse`, `SessionStart`, `SessionEnd`.
2. In `config.toml`:

   ```toml
   [[hooks]]
   event = "PostToolUse"
   tool = "Edit"
   command = "git diff --stat"
   ```

3. Hook receives env vars: `CLIDO_TOOL_NAME`, `CLIDO_TOOL_INPUT`, `CLIDO_TOOL_RESULT`, `CLIDO_SESSION_ID`.

#### 8.1.2 Implement hook executor

1. In `clido-agent/src/events.rs`:
   - After each tool call, check for matching hooks.
   - For matching hooks: spawn subprocess `sh -c {hook_command}` with env vars.
   - Capture output; log at `debug!` level.
   - Hook failures do not stop the agent (logged only).
2. Test: configure a hook → run agent → verify hook fires.

#### 8.1.3 Emit progress events (PostToolUse)

1. Match observed JSONL: emit `SessionLine::Progress { data: { type: "hook_progress", hookEvent: "PostToolUse", hookName: "PostToolUse:{tool_name}", toolUseID, parentToolUseID } }` after each tool.
2. This makes session files compatible with tools that read Claude-format JSONL.

---

### Milestone 8.2 — JSON and Stream-JSON Output

**Dependency:** 3.5 complete.

**Reference:** Schema, `exit_status` taxonomy, and stream-json event types (including compaction events) are defined in [cli-interface-specification.md](cli-interface-specification.md) §10.

#### 8.2.1 Implement `--output-format json`

1. Buffer all events; on completion, output a single JSON object. Include an explicit **exit status** so automation (e.g. CI) can distinguish outcomes without parsing `result` or `error`:

   **Exit status taxonomy:** `exit_status` is one of:
   - `completed` — session ended normally with a final answer.
   - `max_turns_reached` — the loop stopped because `max_turns` was hit; `result` may contain partial output.
   - `budget_exceeded` — the loop stopped because `max_budget_usd` was exceeded.
   - `error` — session ended due to an error (provider failure, tool failure, etc.); `error` field has details.

   Example schema:

   ```json
   {
     "type": "result",
     "exit_status": "completed",
     "result": "...",
     "session_id": "...",
     "num_turns": 5,
     "duration_ms": 12345,
     "total_cost_usd": 0.0045,
     "is_error": false,
     "usage": { "input_tokens": 1234, "output_tokens": 567 }
   }
   ```

   When `exit_status` is `max_turns_reached` or `budget_exceeded`, still include `result` (partial output so far) and set `is_error` to `false` or according to whether the run is considered a failure for the caller. When `exit_status` is `error`, set `"is_error": true` and include `"error": "..."`.

2. Document the `exit_status` values and schema in user docs so CI scripts can branch on them.

#### 8.2.2 Implement `--output-format stream-json`

1. Each event emitted as a newline-delimited JSON object:
   - `{ "type": "text", "text": "..." }` for each text chunk.
   - `{ "type": "tool_use", "name": "Bash", "input": {...} }` when tool starts.
   - `{ "type": "tool_result", "name": "Bash", "is_error": false, "content": "..." }` when tool finishes.
   - `{ "type": "result", "exit_status": "completed"|"max_turns_reached"|"budget_exceeded"|"error", ... }` at end. The same `exit_status` taxonomy as 8.2.1 applies.
2. Test: pipe `clido --output-format stream-json` to `jq` → verify each line is valid JSON.

---

### Milestone 8.3 — MCP (Model Context Protocol) Support

**Dependency:** Phase 5 complete.

#### 8.3.1 Implement MCP client

1. MCP tools expose a JSON-RPC interface over stdio or HTTP.
2. In `clido-tools/src/mcp.rs`:
   - `McpClient` struct: connects to an MCP server process (stdio) or HTTP endpoint.
   - `fn list_tools(&self) -> Vec<ToolSchema>` — calls MCP `tools/list`.
   - `fn call_tool(&self, name: &str, input: Value) -> ToolOutput` — calls MCP `tools/call`.
3. In `clido-tools/src/registry.rs`: allow dynamic registration of MCP tools at startup.

#### 8.3.2 Wire MCP config

1. In `config.toml`:

   ```toml
   [[mcp_servers]]
   name = "filesystem"
   command = "npx"
   args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
   ```

2. At startup: spawn configured MCP servers, register their tools.
3. Implement `clido-cli/src/commands/run.rs` MCP startup routine.
4. Tool guidance (Milestone 3.4.3) regenerates to include MCP tools after registration.

#### 8.3.3 MCP security and trust model

MCP servers are external processes that register as tools. They can read files, run commands, or exfiltrate data if malicious or compromised. Clido's permission system (Phase 4.3) must apply to MCP tools; they must not bypass it.

1. **Allowlist:** MCP servers are only loaded if explicitly declared in the user's config (`config.toml` or equivalent). There is no automatic discovery of MCP servers. The list of MCP server names and commands is user-controlled (allowlist).
2. **Permission behavior:** MCP tools are treated like built-in state-changing tools for permission purposes. Default to `AskUser` (Phase 4.3) for any MCP tool call unless the user has added the tool or server to an allow list for the session. Do not treat MCP tools as inherently read-only.
3. **Audit logging:** Every MCP tool call (server name, tool name, input summary) is recorded in the same append-only audit log as built-in tool calls (Phase 7.3). This enables forensics and policy review.
4. **Environment restriction:** When spawning an MCP server process, do not pass through the full environment. Use a restricted environment: only explicitly allowlisted env vars (e.g. `PATH`, `HOME`, or user-configured `mcp_env_allow`) are passed. Strip `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GITHUB_TOKEN`, `AWS_*`, and other sensitive vars unless the user has explicitly allowlisted them for that server in config. Document the allowlist in the config schema.

---

### Milestone 8.4 — `clido doctor` Command

**Dependency:** 4.5 complete.

**Reference:** Doctor output format, check ordering, exit codes (0=all pass, 1=mandatory fail, 2=warnings only), and `--json` output are defined in [cli-interface-specification.md](cli-interface-specification.md) §12.

#### 8.4.1 Implement health check command

**Version-awareness rule:** `clido doctor` must only check capabilities that are present in the currently running binary. Checking for a missing database or an unbuilt feature as if it were a failure confuses users and hides real issues. The check list below is grouped by the release that introduces each check; later releases extend the list.

**V1 check list (canonical):**

| Check | Severity | Description |
|-------|----------|-------------|
| Config file found and parseable | Mandatory | Global config file (e.g. `~/.config/clido/config.toml` or `$CLIDO_CONFIG`) exists and is valid TOML. |
| Active profile's `api_key_env` variable is set | Mandatory | The selected profile's `api_key_env` (e.g. `ANTHROPIC_API_KEY`) is set in the environment and non-empty. |
| Session directory writable | Mandatory | Session storage directory exists and is writable. |
| `pricing.toml` present and not stale (>90 days) | Warning only | Pricing file present (shipped default or user override); if mtime &gt; 90 days ago, report warning. |
| POSIX shell available (Windows only) | Warning only | On Windows, a POSIX-compatible shell (bash/WSL/Git Bash) is available for BashTool. |

V1 does not include provider connectivity ping (that is added in V1.5). Optional: Rust/Cargo version from `rustc --version` can be reported as informational in V1.

**V1 checks (implementation):**
- CLI config file readable and valid TOML.
- API key presence for the active profile (`api_key_env`).
- Session storage directory exists and is writable.
- `pricing.toml` present (default or user-override); warn if the file is older than 90 days (see 4.4.1).
- On Windows: POSIX shell availability (warning only).

**V1.5 checks (added when MCP is available):**
- Provider connectivity: simple API ping (model list or lightweight probe) for the configured provider.
- MCP servers listed in config: spawn each and attempt a `ping`; report latency and pass/fail per server.

**V3 checks (added when memory is available):**
- Memory DB (`memories.db`) accessible and schema version matches current binary (see 9.7 schema migrations).
- `fastembed` model cache present; if absent, report download required.

**V4 checks (added when repo index is available):**
- Repo index present and up to date (index mtime < last `git commit` mtime).

1. Print `✓` / `✗` for each check.
2. On any `✗`, print a one-line remediation hint (e.g. `"Run 'export ANTHROPIC_API_KEY=...' to fix."`).
3. Exit code: `0` if all checks pass, `1` if any mandatory check fails, `2` if only optional checks fail.
4. Test: mock a missing API key → verify `✗` for provider key check and exit code `1`.
5. Test: V1 binary does not attempt to open `memories.db` → no `✗` for absent memory DB.

---

### Milestone 8.5 — Shell Completion and Man Pages

**Dependency:** 3.2 complete.

#### 8.5.1 Generate shell completions

1. Add `build.rs` to `clido-cli` using `clap_complete`:

   ```rust
   generate(Bash, &mut app, "clido", &mut std::io::stdout());
   ```

2. Generate completions for: bash, zsh, fish.
3. Install instructions in README.

#### 8.5.2 Generate man page

1. Use `clap_mangen` in `build.rs` to generate `clido.1`.
2. Add `clido man` subcommand that prints the man page.

---

### Milestone 8.6 — Live Plan / Progress Visualization

**Dependency:** 4.8 complete (planner), 3.5 complete (streaming).

**Rationale:** Showing the user what the agent is doing at each step is a major usability win. Both Claude CLI and Cursor display tool progress inline.

#### 8.6.1 Implement live tool progress display

1. In `clido-cli/src/output/streaming.rs`, enhance existing tool status display:
   - When tool starts: print `"⏳ {tool_name}: {input_summary}"` on a new line.
   - When tool ends (success): overwrite/replace with `"✓ {tool_name}: {input_summary} ({duration_ms}ms)"`.
   - When tool ends (error): `"✗ {tool_name}: {input_summary} — {error_preview}"`.
   - Use ANSI escape codes to update in-place when terminal supports it (check `TERM` and `NO_COLOR`).
2. Test: run agent with a multi-step task; verify each tool prints status in sequence.

#### 8.6.2 Implement plan display for planner mode

1. In `clido-cli/src/output/plan_display.rs`:
   - `fn display_plan(graph: &TaskGraph)` — renders the task graph as a numbered list:

     ```
     PLAN
     ────────────────────────────────────
      1. [Glob]  Explore repository structure
      2. [Grep]  Find unsafe code  (depends on: 1)
      3. [Read]  Read affected files  (depends on: 2)
      4. [Edit]  Apply fixes  (depends on: 3)
      5. [Bash]  Run tests  (depends on: 4)
     ```

   - Use Unicode box-drawing chars; degrade gracefully on non-UTF-8 terminals.
2. Implement `fn update_plan_status(id: &str, status: TaskStatus)` for live updates:
   - Pending: ` ○`
   - In progress: `⏳`
   - Done: ` ✓`
   - Failed: ` ✗`
3. Wire into planner execution path (Milestone 4.8.5).
4. Test: mock plan with 5 tasks; verify display updates correctly as each completes.

#### 8.6.3 Add `--verbose` flag for detailed tool I/O

1. `clido --verbose` prints full tool inputs and outputs inline.
2. Without `--verbose`: show only name and a short input summary.
3. Test: verify verbose mode shows full `old_string`/`new_string` for Edit calls.

---

### Milestone 8.7 — Repository Indexing (Optional Advanced Feature)

**Dependency:** Phase 5 complete, `clido-index` crate created in Phase 1.

**Rationale:** For large repositories (>10k files), Glob/Grep over raw files is slow. A pre-built index enables sub-millisecond symbol lookup and semantic search without touching every file.

**Note:** This is gated behind `clido index build` and `config.use_index = true`. The agent works without it; the index is purely a performance and capability enhancement.

#### 8.7.1 Implement file index

1. In `clido-index/src/file_index.rs`:
   - Walk repository using `ignore::WalkBuilder` (respects .gitignore).
   - Store: path, size, mtime, language (from extension).
   - Persist as JSON file: `{data_dir}/index/{sanitized_path}/files.json`.
2. Implement `fn update_incremental()`: only reindex files changed since last build (compare mtime).
3. Test: index a fixture repo; verify all files present; add a file; run incremental update; verify new file appears.

#### 8.7.2 Implement symbol extraction with tree-sitter

1. In `clido-index/src/symbol_index.rs`:
   - Add `tree-sitter` + language grammars as optional dependencies:
     - `tree-sitter-rust`, `tree-sitter-javascript`, `tree-sitter-python`, `tree-sitter-typescript`.
   - For each supported file: parse with tree-sitter, extract:
     - Function definitions (name, file, line range)
     - Struct/class definitions
     - Trait/interface definitions
     - Constants and type aliases
   - Store in SQLite: `symbols(name, kind, file_path, start_line, end_line, language)`.
2. Expose `fn search_symbol(query: &str) -> Vec<SymbolEntry>`.
3. Test: index a known Rust file; search for a function name; verify correct location returned.

#### 8.7.3 Implement full-text search with tantivy

1. In `clido-index/src/search.rs`:
   - Use `tantivy` crate for full-text indexing of file contents.
   - Fields: `path`, `content`, `language`.
   - `fn index_file(path: &Path, content: &str, language: &str)`.
   - `fn search(query: &str, limit: usize) -> Vec<SearchResult>`.
2. Store tantivy index at `{data_dir}/index/{sanitized_path}/tantivy/`.
3. Test: index 100 fixture files; search for a unique token; verify correct file returned.

#### 8.7.4 Add `SymbolSearchTool`, `IndexSearchTool`, and (V3) `SemanticSearch` tool

**Reference:** The **SemanticSearch** tool (V3) is specified in [cli-interface-specification.md](cli-interface-specification.md) §2 and D15. It uses the embedding engine (Phase 5.5.3, fastembed-rs) and repo indexing; signature `SemanticSearch(query, target_directory?, num_results?)`. Implement alongside or after the index tools when V3 memory and indexing are available.

1. `SymbolSearchTool`:
   - Parameters: `query` (string, required), `kind` (string, optional: function/struct/trait), `language` (string, optional).
   - `is_read_only()` → `true`.
   - Calls `symbol_index.search_symbol()`.
2. `IndexSearchTool`:
   - Parameters: `query` (string, required), `limit` (integer, optional, default 10).
   - `is_read_only()` → `true`.
   - Calls `search.search()`.
3. Register both in tool registry (only when index is enabled and built).
4. Add to tool guidance prompt when index tools are available.

#### 8.7.5 Wire index CLI commands

1. `clido index build` — build index for current project.
2. `clido index update` — incremental update.
3. `clido index status` — show index freshness, file count, symbol count.
4. On `clido` startup with `use_index = true`: check if index exists; warn if stale (mtime > 10 min).

---

## Phase 9 — Production Readiness

**Goal:** Stable release, full test coverage, documentation, packaging, telemetry.

---

### Milestone 9.1 — Full Test Coverage

**Dependency:** All previous phases complete.

#### 9.1.1 Unit test coverage targets

1. `clido-core`: ≥ 95% coverage (pure data types, easiest).
2. `clido-tools`: ≥ 90% coverage (test each tool with fixture files).
3. `clido-providers`: ≥ 85% coverage (use `wiremock` for HTTP mocking).
4. `clido-agent`: ≥ 80% coverage (use mock provider and mock tools).
5. `clido-context`: ≥ 85% coverage (token counting, compaction, guidance logic).
6. `clido-storage`: ≥ 85% coverage (JSONL read/write with tempfiles).
7. `clido-memory`: ≥ 85% coverage (both short-term and SQLite long-term).
8. `clido-planner`: ≥ 80% coverage (task graph validation, topological sort, executor).
9. `clido-index`: ≥ 75% coverage (file walk, symbol extraction, tantivy integration).

#### 9.1.2 End-to-end integration tests

1. Test against real API (CI, needs secret): `cargo test --features integration`.
2. Scenarios:
   - `"Create a hello world Rust file"` → verify `main.rs` created with valid Rust.
   - `"Fix the syntax error in tests/fixtures/broken.rs"` → verify file fixed.
   - `"What files are in this project?"` → verify Glob/Read used, correct answer returned.
   - Session resume: run task halfway, kill, resume → verify completion.
   - Memory: save memory in session 1; verify it appears in session 2 context.
   - Planner: run with `--planner` flag; verify plan displayed and executed.

#### 9.1.3 Property-based tests

1. Use `proptest` for:
   - Token counting: any valid UTF-8 string → count never panics.
   - Edit tool: any old/new string combination → either succeeds or returns is_error, never panics.
   - Session JSONL: any `SessionLine` → serialization round-trip is lossless.
   - TaskGraph: any list of tasks with random dependencies → `validate()` + `execution_order()` never panics.

---

### Milestone 9.2 — Benchmarks

**Dependency:** 9.1 complete.

#### 9.2.1 Add criterion benchmarks

1. In `clido-core/benches/`:
   - Benchmark JSON serialization of large `Message` arrays (simulating 50-turn context).
2. In `clido-context/benches/`:
   - Benchmark token counting for 100k-token contexts.
3. In `clido-tools/benches/`:
   - Benchmark Grep over a large directory (e.g. the Linux kernel source).
   - Benchmark file read cache hit vs. miss.
4. In `clido-index/benches/`:
   - Benchmark symbol search over a 1000-symbol index.
5. Run: `cargo bench`. Track in CI as regression check.

---

### Milestone 9.3 — Structured Telemetry

**Dependency:** 9.1 complete.

**Rationale:** Production systems need observability: which tools are slow, which model calls fail, what the per-task cost distribution looks like, where retries happen.

#### 9.3.1 Define telemetry event types

1. In `clido-core/src/telemetry.rs`:

   ```rust
   pub struct TelemetryEvent {
       pub session_id: String,
       pub event_type: TelemetryEventType,
       pub timestamp: DateTime<Utc>,
       pub duration_ms: Option<u64>,
       pub metadata: serde_json::Value,
   }

   pub enum TelemetryEventType {
       ToolCall,
       ProviderRequest,
       ProviderRetry,
       SessionStart,
       SessionEnd,
       PlannerCall,
       CompactionTriggered,
       MemorySave,
       MemorySearch,
   }
   ```

#### 9.3.2 Instrument key code paths

1. In `clido-agent/src/executor.rs`:
   - Before tool call: record start time.
   - After tool call: emit `TelemetryEvent { event_type: ToolCall, duration_ms, metadata: { tool_name, is_error } }`.
2. In `clido-providers/src/anthropic.rs`:
   - Emit `ProviderRequest` event with `{ model, input_tokens, output_tokens, cost_usd, duration_ms }`.
   - Emit `ProviderRetry` event on each retry attempt.
3. In `clido-context/src/compaction.rs`:
   - Emit `CompactionTriggered` with `{ tokens_before, tokens_after }`.
4. In `clido-planner/src/planner.rs`:
   - Emit `PlannerCall` with `{ task_count, duration_ms }`.

#### 9.3.3 Telemetry output targets

1. Default: emit to `tracing::info!` spans (already wired to `tracing-subscriber`).
2. Optional: write structured JSON to `{data_dir}/telemetry.jsonl` (append-only).
3. `clido stats` subcommand:
   - Read `telemetry.jsonl` for current session or all sessions.
   - Display: tool call frequency table, average latency per tool, total model cost, retry rate.
4. Test: run a multi-step task; verify `clido stats` outputs correct tool counts.

---

### Milestone 9.4 — Documentation

**Dependency:** 9.2 complete.

**Documentation standard:** All documentation in this milestone — and throughout the project — must meet the bar defined in `devdocs/guides/software-development-best-practices.md` under "Documentation Rules". The short version: everything is documented, always, for both human readers and automated agents (including Clido itself). Documentation is not a finishing step; it is part of each change.

#### 9.4.1 Public API documentation (agent-facing)

1. Add `//!` module-level doc comments to every `lib.rs`. Describe the crate's purpose, key types, and important invariants.
2. Add `///` doc comments to every public type, function, and trait method. Describe what it does, what it assumes, and what it returns or mutates — not just the name restated.
3. Run `cargo doc --workspace --no-deps`; verify no warnings.
4. Add doc tests (`///` examples) for key public functions.
5. Write an `CLIDO.md` at the root of every crate in the workspace. This file is written for AI coding agents (including Clido) working on that crate. Include:
   - The crate's single responsibility.
   - Which files to read first.
   - Key invariants and constraints.
   - Common pitfalls (things that look safe but are not).
   - What tests to run to validate a change.
   - Any special setup required.

#### 9.4.2 User documentation (human-facing)

1. Write `README.md`:
   - Installation (`cargo install clido` or binary releases).
   - Quick start (5 lines to first working agent call).
   - Full configuration reference: **every key in `config.toml` and `.clido/config.toml`**, its type, default, valid values, and a one-line description. No key ships undocumented.
   - Profile system: how to define and switch profiles; example profiles for common use cases (cheap, local, review).
   - Provider setup: API keys, env var names, local model setup (Ollama), OpenRouter setup.
   - All CLI flags with examples, including `--profile`, `--provider`, `--model`.
   - Error message reference: the most common user-visible errors and how to resolve them.
   - Session management.
   - MCP configuration.
   - Memory system usage (including `clido memory prune`).
   - Planner mode.
   - Repo indexing.
   - Self-improvement loop commands (`clido reflect`, `clido improve`).
2. Write `CONTRIBUTING.md`: how to add a new tool, how to add a new provider, how to define a new profile, documentation requirements per change.

#### 9.4.3 Architecture documentation (human-facing)

1. Write `docs/architecture.md`:
   - Module dependency graph (mermaid diagram).
   - Data flow diagram (agent loop → context engine → provider → tool executor).
   - Session JSONL format spec.
   - Tool schema format spec.
   - Task graph JSON schema.
   - Memory DB schema.
   - Config schema: full field-by-field documentation of `config.toml`, `.clido/config.toml`, and `pricing.toml`.
   - Profile system design: how config loading, merging, and profile resolution work end to end.

#### 9.4.4 Ongoing documentation hygiene

The following rules apply continuously, not only during Milestone 9.4:

- **Every new CLI flag** ships with: `--help` text, a `README.md` entry, and an example.
- **Every new config key** ships with: a comment in the default config, a `README.md` entry, and clear validation error message when the value is wrong.
- **Every new error message** is documented: what triggered it, what the user should do.
- **Every new tool** has: a `ToolSchema.description` that an AI agent can use to decide when and how to call it, and a section in `README.md`.
- **Every breaking change** has: a changelog entry and migration instructions before it ships.
- **Stale documentation is a bug.** If a doc update is skipped, the change is not done.

---

### Milestone 9.5 — Packaging and Distribution

**Dependency:** 9.3 complete.

#### 9.5.1 Cross-platform builds

1. Add GitHub Actions matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`.
2. Build static binaries with `musl` target on Linux: `x86_64-unknown-linux-musl`.
3. Build universal binary on macOS: `x86_64-apple-darwin` + `aarch64-apple-darwin`, then `lipo`.
4. Upload release artifacts to GitHub Releases.

**Windows strategy for `BashTool`:**

`BashTool` spawns shell commands via `std::process::Command`. The behavior differs significantly on Windows:

- Windows has no `/bin/bash` by default. Spawning `bash -c "..."` fails unless Git Bash, WSL, or MSYS2 is installed.
- `std::process::Command` on Windows uses `cmd.exe` by default; many Unix commands (`ls`, `cat`, `grep`) do not exist.
- The CI Windows build **must not silently pretend `BashTool` works** when it does not.

**Decision: Windows is a Tier 2 platform for V1; `BashTool` is explicitly degraded.**

Implementation:

1. At startup on Windows, detect whether a POSIX-compatible shell is available (`bash`, `sh` via WSL, or Git Bash). Detection order: `bash.exe` on `PATH`, then `wsl bash`. Store result in a static `OnceCell<Option<PathBuf>>`.
2. If no POSIX shell found: `BashTool::execute()` returns `is_error: true` with message: `"BashTool is not available on Windows without a POSIX-compatible shell (bash/WSL/Git Bash). Install Git Bash or WSL to enable shell commands."`. Do not panic.
3. If a shell is found: run commands through it (e.g. `bash.exe -c "{command}"`).
4. `clido doctor` on Windows adds a V1 check: verify POSIX shell availability; print a clear warning (not a failure) if absent.
5. Document this limitation in the user-facing docs and README under "Platform Support."
6. CI Windows build: run the full test suite with `BashTool` in degraded mode (mock/skip shell-dependent tests); all other tools must pass.
7. Long-term (V2+): consider a `PowerShell` execution mode as a first-class alternative for Windows users.

#### 9.5.2 Homebrew formula

1. Create `clido.rb` formula:
   - Download release tarball.
   - `bin.install "clido"`.
   - Install shell completions.
2. Publish to a Homebrew tap.

#### 9.5.3 `cargo install` support

1. Verify `cargo install clido` (from crates.io) works.
2. Publish `clido-core`, `clido-tools`, `clido-providers`, `clido-context`, `clido-storage`, `clido-memory`, `clido-planner`, `clido-index`, `clido-agent`, `clido-cli` to crates.io in dependency order.

---

### Milestone 9.6 — Production Hardening

**Dependency:** 9.4 complete.

#### 9.6.1 Memory leak and resource auditing

1. Run `valgrind` (Linux) or `Instruments` (macOS) on a long agent session.
2. Verify no unbounded memory growth over 100 turns.
3. Verify all spawned subprocesses are reaped.
4. Verify all file handles are closed after tool execution.

#### 9.6.2 Fuzz testing

1. Use `cargo-fuzz` on:
   - `SessionReader::load()` — fuzz JSONL input.
   - `EditTool::execute()` — fuzz file content + old/new strings.
   - `GrepTool::execute()` — fuzz regex patterns.
   - `TaskGraph::validate()` — fuzz task graph JSON.
2. Run for 1 hour; fix any panics or crashes found.

#### 9.6.3 Security audit

1. Run `cargo audit` to check for known CVEs in dependencies.
2. Run `cargo deny check` with a policy file (`licenses`, `bans`, `advisories`).
3. Review all `unsafe` blocks (should be zero outside FFI if using pure Rust).
4. Review all subprocess spawning for injection vulnerabilities.
5. **Project instructions prompt injection:** Verify that `CLIDO.md` / `CLAUDE.md` loading uses trust-on-first-use and size limits (Phase 3.4.2). Confirm that a malicious repo with adversarial instructions cannot hijack Clido without user approval on first load.

#### 9.6.4 Release checklist

- [ ] All tests pass on all platforms
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo audit` clean
- [ ] `cargo deny check` clean
- [ ] No `unwrap()` or `expect()` in non-test code (replaced with `?` or explicit error handling)
- [ ] Version bumped in all `Cargo.toml` files
- [ ] `CHANGELOG.md` updated with all user-visible changes since the previous release (see 9.7.3)
- [ ] GitHub Release created with binaries
- [ ] Homebrew formula updated
- [ ] crates.io published

---

### Milestone 9.7 — Schema Versioning and Migrations

**Dependency:** 3.3 (session JSONL), 5.5 (memory SQLite).

**Rationale:** Clido stores data in two durable formats: JSONL session files and the SQLite memory database. Without explicit schema versions and migration logic, any structural change (new field, renamed column, dropped table) silently breaks older data. Users upgrading Clido must not lose sessions or memories.

#### 9.7.1 Session JSONL schema versioning

1. Add a `schema_version: u32` field to the session metadata header (first line of every JSONL file), starting at `1`.
2. When reading a session file:
   - If `schema_version` is absent: treat as version `0` (legacy).
   - If `schema_version > CURRENT_VERSION`: refuse to load the file with a clear error: `"Session file was written by a newer version of Clido (schema {found}, binary supports up to {max}). Upgrade Clido to read this session."`.
   - If `schema_version < CURRENT_VERSION`: apply forward migration transforms before loading. Define a `SessionMigration` trait with a `migrate(old: Value) -> Value` method; register migrations by `(from_version, to_version)`.
3. Migrations are cumulative and run in order. Each migration is a pure function over raw JSON values (no side effects, composable, testable).
4. Bump `schema_version` whenever a breaking or additive field change is made to `SessionLine` or session metadata. Document each bump in `CHANGELOG.md`.
5. Test: load a V0 session file (fixture) with the V1 reader → verify migration is applied and all fields parse correctly.

#### 9.7.2 Memory SQLite schema versioning

1. Add a `user_version` pragma to the SQLite database (SQLite's built-in schema version field: `PRAGMA user_version = N`). Start at `1` when the schema is first created.
2. At startup, read `PRAGMA user_version`. If it is lower than the compiled-in `CURRENT_DB_VERSION`, run migrations in order. Each migration is a Rust closure that runs within a transaction (atomically).
3. Migration structure:
   ```rust
   struct DbMigration {
       from_version: u32,
       description: &'static str,
       sql: &'static str,   // DDL / DML to apply
   }
   const MIGRATIONS: &[DbMigration] = &[
       DbMigration { from_version: 0, description: "initial schema", sql: include_str!("migrations/001_initial.sql") },
       // future migrations appended here
   ];
   ```
4. After running all migrations, update `PRAGMA user_version` to `CURRENT_DB_VERSION`.
5. If `user_version > CURRENT_DB_VERSION`: refuse to open and return: `"Memory database was written by a newer version of Clido. Upgrade Clido or run 'clido memory reset' to start fresh."`.
6. Store migration SQL in `clido-memory/src/migrations/NNN_description.sql` files. These are embedded at compile time via `include_str!`.
7. Test: create a V0 database (no `user_version`), open with V1 reader → verify schema is migrated and version is updated. Open a V999 database → verify error and no data loss.

#### 9.7.3 CHANGELOG discipline

Every merged PR that changes user-facing behavior, CLI flags, config schema, session schema, or DB schema **must** include a `CHANGELOG.md` update as part of the PR. Add a CI check (e.g. a lint script or `git diff --name-only` in the action) that fails the PR if `CHANGELOG.md` was not touched in PRs labelled `feature`, `fix`, `breaking`, or `schema-change`. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## Dependency Map

```
Phase 1 (Foundation)
  └─ Phase 2 (PoC)
       └─ Phase 3 (MVA)
            ├─ Phase 4 (Feature Expansion)
            │    ├─ Phase 5 (Reliability)
            │    │    ├─ Phase 6 (Performance)
            │    │    ├─ Phase 7 (Security)
            │    │    └─ Phase 8 (DX)
            │    │         └─ Phase 9 (Production)
            │    └─ Phase 7 (Security)
            └─ Phase 4

Task-level dependencies (critical path):
1.1 Workspace Init
  → 1.2 Core Types
    → 1.3 Tracing
      → 2.1 Anthropic Provider
        → 2.2 Bash Tool (Tool trait + is_read_only)
          → 2.3 Agent Loop PoC
            → 3.1 All Tools
              → 3.2 Clap CLI
                → 3.3 Session Storage
                  → 3.4 Config Loading + Tool Guidance Prompts (3.4.3)
                    → 3.5 Streaming (partial JSON assembly)
                      → 4.1 Multi-Provider
                      → 4.2 Context Engine
                      → 4.3 Permissions
                        → 4.5 Plan Mode
                      → 4.6 Parallel Tools + Semaphore (4.6.3)
                        → 4.7 Subagents
                          → 4.8 Task Graph / Planner
                            → 8.6 Live Plan Visualization
                      → 5.1 Error Handling
                        → 5.2 Session Recovery
                        → 5.3 Graceful Shutdown
                        → 5.5 Memory System
                          → 6.x Performance
                            → 6.4 File Read Cache
                          → 7.x Security
                          → 8.x DX
                            → 8.3 MCP
                            → 8.7 Repo Indexing (optional)
                              → 9.x Production
                                → 9.3 Telemetry
```

---

## Recommended Crates Reference

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.x | Async runtime (full features) |
| `clap` | 4.x | CLI argument parsing (derive feature) |
| `serde` + `serde_json` | 1.x | Serialization |
| `thiserror` | 1.x | Structured error types |
| `anyhow` | 1.x | Ad-hoc error chaining |
| `tracing` + `tracing-subscriber` | 0.1/0.3 | Structured logging and telemetry spans |
| `reqwest` | 0.12 | HTTP client (json + stream features) |
| `tokio-stream` | 0.1 | Async streaming |
| `futures` | 0.3 | Future combinators (join_all, etc.) |
| `uuid` | 1.x | Session ID generation (v4) |
| `chrono` | 0.4 | Timestamps (serde feature) |
| `directories` | 5.x | XDG/platform config and data paths |
| `glob` | 0.3 | File glob matching |
| `ignore` | 0.4 | gitignore-aware directory walking |
| `regex` | 1.x | Regular expression matching |
| `grep` | 0.3 | ripgrep-family grep engine |
| `similar` | 2.x | Unified diff generation for Edit tool |
| `tiktoken-rs` | 0.5 | Token counting (cl100k/o200k, approximate) |
| `async-trait` | 0.1 | Async trait methods |
| `toml` | 0.8 | Config file parsing |
| `rustyline` | 14.x | Interactive REPL with history |
| `wiremock` | 0.6 | HTTP mock server for tests |
| `jsonschema` | 0.17 | JSON Schema validation for tool inputs |
| `proptest` | 1.x | Property-based testing |
| `criterion` | 0.5 | Benchmarking |
| `clap_complete` | 4.x | Shell completion generation |
| `clap_mangen` | 0.2 | Man page generation |
| `rusqlite` or `sqlx` | latest | SQLite for long-term memory |
| `sqlite-vec` | 0.1.x | Vector similarity search extension for SQLite (pure C, no server) |
| `fastembed` | 4.x | Local ONNX-based embedding generation (BGE-small, no API required) |
| `lru` | 0.12 | LRU cache for file reads |
| `tree-sitter` | 0.22 | Code parsing for symbol extraction (optional) |
| `tree-sitter-rust` | 0.21 | Rust grammar for tree-sitter (optional) |
| `tantivy` | 0.22 | Full-text search for repo indexing (optional) |
| `seccomp` | 0.1 | Linux syscall sandboxing (optional) |

---

*Generated from: `devdocs/REPORT.md`, `devdocs/ARTIFACTS.md`, `devdocs/Instructions.md`*
*Updated: 2026-03-16 — added bounded concurrency (4.6.3), tool guidance prompts (3.4.3), subagent architecture (4.7), task graph planner (4.8), memory system (5.5), file read LRU cache (6.4), live plan visualization (8.6), repo indexing (8.7), structured telemetry (9.3); prompt caching (4.2.4), hybrid memory with sqlite-vec + fastembed (5.5.3–5.5.5), relevance-based memory injection (5.5.5); weakness audit round 2 (5.6 edit safety, 5.2.3 stale-file detection, 3.4.2 CLIDO.md trust, 7.1.2 sandbox deprecation, 8.3.3 MCP trust, 5.1.0 shared retry, 4.7.2 subagent budget, 4.2.1/4.2.2 token safety, 4.8.3a planner checkpoint, 5.5.6 memory pruning, 4.4.1 pricing.toml, 8.2.1 exit status, 4.2.3 pinned context); provider/model switching (3.4.1 profiles + project config, --profile/--provider CLI flags, API key validation, 4.1.5 factory); weakness audit round 3 (4.1.5 unsupported-provider guard + model tool-use check, 4.3.3 serialized AskUser permission gate, 4.2.3 compaction failure fallback, 5.4.2 per-release coverage floors + V1 critical-path tests, 9.5.1 Windows BashTool strategy, 5.5.6 per-session memory write cap, 9.7 schema versioning + migrations, 9.6.4 CHANGELOG CI enforcement, 4.4.1 pricing.toml staleness + update-pricing command, 4.1.3 offline mode, 6.4.1 mtime+hash dual-check cache, 8.4.1 version-aware doctor)*
