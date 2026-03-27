# Clido vs OpenCode: Deep Competitive Analysis

## 5-Pass Reverse Engineering & Strategic Roadmap

> **Produced**: 2026-03-27  
> **Method**: 5-pass comparative reverse engineering of both full codebases  
> **Scope**: Full source analysis — every .rs and .ts file examined

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Methodology](#2-methodology)
3. [Deep System Breakdown](#3-deep-system-breakdown)
4. [Category-by-Category Comparison](#4-category-by-category-comparison)
5. [Best Practices to Adopt from OpenCode](#5-best-practices-to-adopt-from-opencode)
6. [Clido Strengths](#6-clido-strengths)
7. [Missing Features](#7-missing-features)
8. [Critical Weaknesses](#8-critical-weaknesses)
9. [Improvement Roadmap](#9-improvement-roadmap)
10. [Path to Superiority](#10-path-to-superiority)
11. [Final Critical Reflection](#11-final-critical-reflection)

---

## 1. Executive Summary

Clido and OpenCode are both AI coding agents, but they sit in fundamentally different architectural tiers. **Clido is a focused, fast, CLI-first tool** built in Rust with a clean agent loop, solid tool coverage, and a unique optional DAG planner. **OpenCode is a multi-surface platform** built in TypeScript/Bun using the Effect functional framework, SQLite-backed sessions, an event bus, a plugin ecosystem, LSP integration, snapshot/revert, session sharing, and both TUI and full GUI surfaces.

The gap is significant but not insurmountable — and Clido has meaningful advantages in startup speed, safety (sandbox, secret scanning, audit), the DiffReview permission mode, YAML workflows, and a more scriptable/automation-friendly design. The path to superiority requires closing four critical gaps: **permission granularity**, **context compaction quality**, **session persistence backend**, and **LSP/diagnostics integration** — while doubling down on Clido's unique strengths.

**Key finding**: OpenCode's biggest weakness is architectural complexity: the Effect framework, Bun-specific runtime, multi-package monorepo, and GraphQL-style event bus make it hard to extend and reason about. Clido's straightforward Rust crates are significantly easier to maintain and extend. This architectural clarity is a strategic asset.

---

## 2. Methodology

### Pass Structure

Each pass built on the previous, going deeper into code, behavior, and implications:


| Pass       | Focus                      | New Depth Added                                               |
| ---------- | -------------------------- | ------------------------------------------------------------- |
| **Pass 1** | Structural inventory       | Crate/package maps, entry points, basic flows                 |
| **Pass 2** | Tool system deep dive      | Every tool impl, permission gates, output handling            |
| **Pass 3** | State & context management | History management, compaction strategies, token accounting   |
| **Pass 4** | Architecture & quality     | Effect layer design, Rust ownership patterns, complexity cost |
| **Pass 5** | UX simulation & edge cases | Real usage flows, failure modes, hidden friction points       |


### Sources Examined

**Clido (Rust)**:

- `crates/clido-agent/src/agent_loop.rs` — full agent loop, permission model, tool gating
- `crates/clido-agent/src/sub_agent.rs` — worker/reviewer spawning
- `crates/clido-cli/src/repl.rs` — REPL loop, slash commands, session recording
- `crates/clido-cli/src/agent_setup.rs` — full config assembly, provider routing, sub-agent wiring
- `crates/clido-core/src/{config,types,error,model_prefs,pricing}.rs` — type system
- `crates/clido-tools/src/` — all 17+ tool implementations
- `crates/clido-providers/src/{anthropic,openai,provider}.rs` — provider adapters
- `crates/clido-planner/src/` — DAG planner (graph, executor, storage, parser)
- `crates/clido-workflows/src/` — YAML workflow engine
- `crates/clido-context/src/` — token estimation, context assembly, rules loading
- `crates/clido-memory/src/` — long-term memory store
- `crates/clido-storage/src/` — session/audit JSONL
- `Cargo.toml` (workspace), `README.md`, `.clido/`

**OpenCode (TypeScript/Bun)**:

- `packages/opencode/src/agent/agent.ts` — agent registry, permission defaults per agent
- `packages/opencode/src/session/{index,processor,compaction,llm,system}.ts` — full agent loop
- `packages/opencode/src/tool/` — all 20+ tool implementations
- `packages/opencode/src/permission/index.ts` — granular permission evaluation
- `packages/opencode/src/config/config.ts` — multi-level config with JSONC
- `packages/opencode/src/provider/` — Vercel AI SDK integration, 15+ providers
- `packages/opencode/src/snapshot/index.ts` — git-based snapshot system
- `packages/opencode/src/worktree/index.ts` — git worktree management
- `packages/opencode/src/skill/index.ts` — SKILL.md discovery
- `packages/opencode/src/storage/` — SQLite-backed persistence
- `packages/opencode/src/bus/` — pub/sub event bus
- `packages/opencode/AGENTS.md` — codebase conventions

---

## 3. Deep System Breakdown

### 3.1 Clido Architecture

```
clido-cli         ← Entry point: REPL, single-shot, TUI, subcommands
    ↓
clido-agent       ← AgentLoop (run/run_next_turn), SubAgent
    ↓
clido-providers   ← AnthropicProvider, OpenAICompatProvider
clido-tools       ← ToolRegistry + 17 tools (trait-based)
clido-context     ← Token estimation, history compaction, rules loading
clido-memory      ← Keyword-based long-term memory (SQLite FTS5)
clido-storage     ← JSONL session files, AuditLog
clido-planner     ← DAG planner (TaskGraph, PlanExecutor) — optional
clido-workflows   ← YAML workflow engine — optional
clido-core        ← Types, errors, config, pricing
```

**Execution model**: Synchronous loop — `run()` calls provider, gets response, executes tools, loops. Parallel tool execution via `tokio::spawn` + `Semaphore` (bounded by `max_parallel_tools=4`). Streaming is supported at provider level but the loop currently uses `complete()` (non-streaming) by default.

**State**: `Vec<Message>` in memory. Session written to JSONL file incrementally. No database. No event bus.

**Permission model**: 4 modes (`Default`, `AcceptAll`, `PlanOnly`, `DiffReview`). Binary ask/deny per tool invocation. No per-file-pattern rules.

### 3.2 OpenCode Architecture

```
CLI/TUI Entry (packages/opencode/src/index.ts)
    ↓
Effect Runtime (Layer composition)
    ↓
Session.Service     ← SQLite-backed sessions, messages, parts
Agent.Service       ← Agent registry with per-agent permissions
Permission.Service  ← Async deferred permission resolution
ToolRegistry.Service← Tool initialization, model-specific filtering
Config.Service      ← Multi-level JSONC config (managed > global > project)
Provider.Service    ← Vercel AI SDK, 15+ providers
Snapshot.Service    ← Git-based snapshot/revert
Worktree.Service    ← Git worktree per session
LSP.Service         ← Language server protocol integration
Plugin.Service      ← Plugin loader, hook system
Skill.Service       ← SKILL.md discovery
Bus               ← Pub/sub event routing
```

**Execution model**: Effect-based async, per-session processor (`SessionProcessor.create()`). Streams LLM output and processes events in real-time (text-delta, tool-call, tool-result). Messages and parts stored to SQLite on every event.

**State**: SQLite database per workspace. Messages, parts, sessions all persisted immediately. Parent/child session graph for sub-agents. Bus events notify TUI/app in real-time.

**Permission model**: Ruleset-based — array of `{permission, pattern, action}` rules. Wildcard matching, last rule wins. Per-agent default rulesets. Async deferred resolution (UI shows permission dialog, blocks tool until answered).

---

## 4. Category-by-Category Comparison

### 4.1 Core Agent Capabilities


| Capability                 | Clido                                      | OpenCode                                        | Delta                      |
| -------------------------- | ------------------------------------------ | ----------------------------------------------- | -------------------------- |
| **Reactive agent loop**    | ✅ Clean `run()`/`run_next_turn()`          | ✅ `SessionProcessor.process()`                  | Comparable                 |
| **Streaming responses**    | ⚠️ Provider-level only; loop uses batch    | ✅ Full event-stream processing                  | OpenCode wins              |
| **Multi-agent/sub-agents** | ✅ Worker + Reviewer via SpawnTools         | ✅ Any agent can spawn sub-agents                | OpenCode broader           |
| **File read**              | ✅ + cache + stale detection                | ✅ + FileTime stale detection                    | Comparable                 |
| **File write**             | ✅ + secret scan + parent mkdir             | ✅ + LSP diagnostics                             | OpenCode shows diagnostics |
| **File edit**              | ✅ 3-tier matching (exact/normalized/fuzzy) | ✅ Advanced (cline+gemini source) + LSP feedback | Comparable/close           |
| **Bash**                   | ✅ Sandbox (macOS/Linux), secret env strip  | ✅ Standard, Tree-Sitter parse                   | Clido wins (sandboxing)    |
| **Grep/Glob**              | ✅                                          | ✅                                               | Comparable                 |
| **ls / directory listing** | ❌ No dedicated ls tool                     | ✅ `ls` tool                                     | OpenCode wins              |
| **Multi-file edit**        | ❌ No multiedit/apply_patch                 | ✅ `apply_patch` (unified diff, GPT-4)           | OpenCode wins              |
| **LSP integration**        | ❌ None                                     | ✅ Experimental: go-to-def, refs, hover, symbols | OpenCode wins              |
| **Todo management**        | ❌ No dedicated tool                        | ✅ `todowrite` tool + UI display                 | OpenCode wins              |
| **Skill system**           | ❌ None                                     | ✅ SKILL.md discovery + invocation               | OpenCode wins              |
| **Image input**            | ✅ CLI image attach                         | ✅                                               | Comparable                 |
| **Snapshot/revert**        | ❌ Checkpoint system exists but limited     | ✅ Full git-based snapshot + revert              | OpenCode wins              |


### 4.2 Tooling & Permission Model

**Clido Permission Model** (binary):

```
PermissionMode::Default    → ask per-tool-invocation
PermissionMode::AcceptAll  → never ask
PermissionMode::PlanOnly   → block all write tools
PermissionMode::DiffReview → show diff before every write/edit ← UNIQUE ADVANTAGE
```

Strengths: Simple, predictable, scriptable (env var `CLIDO_PERMISSION_MODE`).  
Weaknesses: No per-file granularity. Can't say "allow edits to `*.ts` but ask for `*.sql`". Can't distinguish "allow globally" vs "allow once". No "deny with feedback" for the agent to retry.

**OpenCode Permission Model** (rule-based):

```typescript
// Per-file rules
{ permission: "edit", pattern: "src/**/*.ts", action: "allow" }
{ permission: "edit", pattern: "*.sql",        action: "ask"   }
{ permission: "read", pattern: "*.env",        action: "ask"   }
{ permission: "bash", pattern: "*",            action: "allow" }

// Async deferred resolution — UI blocks until user responds
// Three reply types: "once", "always", "reject"
// "reject" sends CorrectedError with feedback text to agent
```

Strengths: Fine-grained, agent-customizable, per-session overrides, feedback on rejection.  
Weaknesses: Complex to understand/configure. Rule order matters (last wins). UX for power-users can be overwhelming.

**Delta**: OpenCode's permission model is architecturally superior. Clido needs per-file-pattern rules and "reject with feedback" to close this gap. The `DiffReview` mode is Clido's unique advantage here — OpenCode has no equivalent.

### 4.3 User Flows

#### Onboarding Flow

**Clido**:

1. `cargo install` or binary download
2. First run → interactive `clido init` wizard (provider, API key, writes `~/.config/clido/config.toml`)
3. `clido "your task"` or `clido` for REPL

**OpenCode**:

1. `curl -fsSL https://opencode.ai/install | bash`
2. First run → config auto-detected, provider auth flow
3. `opencode` → TUI opens with model selector and auth prompts

**Delta**: OpenCode's first-run UX is smoother (no separate init step). Clido's `init` wizard is functional but requires an extra explicit step.

#### Running Tasks

**Clido REPL flow**:

```
$ clido
clido> fix the null pointer in user.rs
[spinner: Read user.rs] [spinner: Edit user.rs] [spinner: Bash cargo check]
Fixed: replaced `user.name` with `user.name.clone()` on line 42.
Cost: 3 turns, $0.0021

clido> /cost
Session: 3 turns, $0.0021 total
clido> /exit
```

**OpenCode TUI flow**:

```
$ opencode
[Full-screen TUI loads]
[Model selector at top, session list on left, conversation on right]
[Type prompt in bottom text area, Ctrl+Enter to submit]
[Tool calls shown in real-time with expandable diffs]
[Permission dialogs appear inline]
[/sessions to switch, Tab for model change]
```

**Delta**: OpenCode's TUI is significantly richer — real-time streaming visible in UI, inline diffs, expandable tool calls, model switcher. Clido's REPL is functional but minimal. **This is the largest visible UX gap.**

#### Handling Failures

**Clido**: Tool errors returned as error ToolResult to agent, which reasons about them. No retry logic at tool level (provider has exponential backoff). `ClidoError::Interrupted` propagates cleanly.

**OpenCode**: 

- "Doom loop" detection (3+ consecutive failed tool calls → asks permission to continue)
- Provider retry with exponential backoff
- `CorrectedError` with user feedback fed back to agent
- Compaction on context overflow

**Delta**: OpenCode's failure handling is more sophisticated. The doom-loop detection is a practical safeguard that Clido lacks entirely.

### 4.4 TUI / CLI UX


| Feature                  | Clido                                       | OpenCode                                    |
| ------------------------ | ------------------------------------------- | ------------------------------------------- |
| **Interface type**       | Plain REPL (stderr prompt, stdout output)   | Full-screen TUI (React/Ink-style rendering) |
| **Streaming visibility** | ⚠️ Spinner only                             | ✅ Full real-time stream                     |
| **Tool progress**        | ✅ Tool name in spinner                      | ✅ Expandable tool call panels               |
| **Diff display**         | ✅ DiffReview mode shows diff                | ✅ Inline diff always shown                  |
| **Session list**         | ✅ `/sessions` slash command                 | ✅ Session sidebar panel                     |
| **Model switching**      | ⚠️ Restart required                         | ✅ In-session model switch                   |
| **Permission dialogs**   | ✅ Stdin Y/N/edit/all                        | ✅ Inline dialog with options                |
| **Slash commands**       | ✅ /help /cost /sessions /resume /mode /exit | ✅ Rich slash command menu                   |
| **@agent mentions**      | ❌                                           | ✅ `@explore`, `@general`, etc.              |
| **Todo panel**           | ❌                                           | ✅ Real-time todo tracking visible           |
| **Color support**        | ✅ ANSI color detection                      | ✅ Full color + theming                      |
| **Keyboard nav**         | ❌ (REPL only)                               | ✅ Full keybindings system                   |


**Critical finding**: Clido's UX is essentially 2000s-era REPL while OpenCode has a modern full-screen TUI. This is the most visible competitive gap to a user evaluating both tools.

### 4.5 Planning & Task Execution

**Clido's Planner** (unique feature):

```rust
// clido-planner: Optional DAG planner
pub struct TaskGraph {
    pub tasks: Vec<TaskNode>,   // Nodes
    pub goal: String,
}
pub struct TaskNode {
    pub id: TaskId,
    pub description: String,
    pub depends_on: Vec<TaskId>,  // Edges
    pub tools: Option<Vec<String>>,
    pub complexity: Complexity,
    pub status: TaskStatus,
}

// Activated with: clido --planner "build auth system"
// Creates JSON task graph → validates (no cycles) → executes in dependency order
// Falls back to reactive loop on invalid graph
```

**Important finding**: The `PlanExecutor` currently processes parallel batches **sequentially**. The code comment acknowledges: "Run batch tasks sequentially for now (parallel execution is a future optimisation)." This means `--planner` adds overhead without true parallelism yet.

**OpenCode's Plan Mode**:

```typescript
// plan agent: dedicated primary agent
// - Denies all edit tools
// - Writes plan to .opencode/plans/<slug>.md
// - plan_exit tool: agent calls this when plan complete
//   → prompts user "Switch to build agent?"
//   → if yes: creates new message routed to "build" agent

// No DAG structure — freeform markdown plan
// @general subagent: can run multiple tasks in parallel via task tool
```

**Delta**: Clido's DAG planner is a genuine architectural differentiator — structured dependency tracking, complexity metadata, topological execution. OpenCode uses freeform markdown plans. However, Clido's planner isn't yet leveraging its parallel execution capability. Once parallelism is enabled, this becomes a strong advantage.

### 4.6 Prompt / Input Handling

**Clido**:

- Single line at a time (readline-style)
- `//` prefix to escape `/` commands
- Image input via `--image` flag (base64 encoded)
- JSON output format (`--output-format json`)
- `stream-json` format (reserved for V2)
- No input queue — strictly one prompt at a time
- No `@agent` mention syntax

**OpenCode**:

- Multi-line textarea in TUI
- `@agent` mentions route to specific agents
- `@file` references include file content
- Plugin hook: `experimental.chat.messages.transform` (can mutate messages before send)
- Input parts system: text, file, image
- Session-level system prompt override

**Delta**: OpenCode's input handling is more capable. The `@mention` routing and `@file` references are significant quality-of-life features. Clido's image support is solid but buried in a flag.

### 4.7 Context Management

**Clido Context Management**:

```rust
// Token estimation: (char_count / 4) — very rough heuristic
pub fn estimate_tokens_str(s: &str) -> u32 {
    (s.chars().count() as u32).div_ceil(4)
}

// Compaction trigger: tokens > max_context_tokens * threshold (default 0.75)
// Compaction method: removes all but last N messages (sliding window)
// Dedup: removes duplicate file reads from history

// Config:
pub max_context_tokens: Option<u32>,
pub compaction_threshold: Option<f64>,  // default 0.75
```

**Critical weakness**: The compaction implementation uses a simple sliding-window approach that **throws away potentially critical context**. There is no summary generation — old messages are dropped, which can confuse the agent about decisions made earlier.

**OpenCode Context Management** (two-strategy):

```typescript
// Strategy 1: Pruning (proactive, per-turn)
// - Goes backward through old tool outputs
// - Removes tool results older than the last 40K tokens worth
// - Protected tools (e.g., "skill") are never pruned
// - Prune threshold: > PRUNE_MINIMUM (20K) tokens freed

// Strategy 2: Compaction (on overflow)
// - Uses dedicated "compaction" agent to summarize context
// - Compaction prompt produces structured summary (Goal/Instructions/Discoveries/Accomplished/Files)
// - Replay: re-runs the last user message after compaction
// - Media stripping: removes images on overflow
// - Prevents token accounting bugs: Anthropic vs OpenAI use different token counting

// Token accounting — provider-aware:
// OpenRouter/OpenAI: inputTokens includes cached (subtract for cost calc)
// Anthropic/Bedrock: inputTokens excludes cached (add cache tokens separately)
```

**Delta**: OpenCode's two-strategy approach (pruning + summary compaction) is architecturally superior to Clido's simple window truncation. The summary compaction keeps the agent aware of past decisions; pruning reduces token count without information loss. Clido's approach will cause agent confusion on long tasks.

### 4.8 Configuration & Providers

**Clido Config** (`~/.config/clido/config.toml`):

```toml
default_profile = "default"

[profiles.default]
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"

[profiles.fast]
provider = "openai"
model = "gpt-4o-mini"

[agents]
[agents.main]
provider = "anthropic"
model = "claude-opus-4-5"

[agents.worker]
provider = "openai"
model = "gpt-4o-mini"

[agents.reviewer]
provider = "anthropic"
model = "claude-3-5-haiku"

[roles]
fast = "claude-3-5-haiku-20241022"
reasoning = "claude-opus-4-5"

[hooks]
pre_tool_use = "echo 'starting $CLIDO_TOOL_NAME'"
post_tool_use = "notify-send done"
```

**OpenCode Config** (`~/.config/opencode/config.jsonc`):

```jsonc
{
  "model": "anthropic/claude-sonnet-4-5",
  "provider": {
    "anthropic": { "apiKey": "sk-ant-..." }
  },
  "agent": {
    "build": { "model": "anthropic/claude-opus-4-5" },
    "explore": { "model": "anthropic/claude-haiku-4-5" }
  },
  "permission": {
    "edit": { "**/*.sql": "ask" }
  },
  "plugin": ["@opencode/smart-git"],
  "instructions": ["AGENTS.md", "docs/spec.md"],
  "experimental": {
    "lsp": true,
    "batch_tool": true
  }
}
```

**Delta**: Both configs are well-designed. OpenCode's JSONC is more ergonomic (comments allowed, JSON familiarity). Clido's TOML is type-safe and more readable for complex structures. OpenCode has plugin declarations in config (powerful but adds complexity). Clido's `[roles]` named role system is elegant and unique.

**Providers supported**:


| Provider            | Clido | OpenCode |
| ------------------- | ----- | -------- |
| Anthropic           | ✅     | ✅        |
| OpenAI              | ✅     | ✅        |
| OpenRouter          | ✅     | ✅        |
| Azure OpenAI        | ❌     | ✅        |
| Google (Gemini)     | ❌     | ✅        |
| Google Vertex       | ❌     | ✅        |
| AWS Bedrock         | ❌     | ✅        |
| Groq                | ❌     | ✅        |
| Mistral             | ✅     | ✅        |
| MiniMax             | ✅     | ❌        |
| Alibaba (DashScope) | ✅     | ❌        |
| GitLab              | ❌     | ✅        |
| GitHub Copilot      | ❌     | ✅        |
| xAI (Grok)          | ❌     | ✅        |
| Ollama/Local        | ✅     | ✅        |


**Delta**: OpenCode supports ~10 more providers. Clido has unique support for MiniMax and Alibaba Cloud (useful in Chinese market). Both support OpenRouter as a universal gateway.

### 4.9 Error Handling & Recovery

**Clido**:

```rust
pub enum ClidoError {
    Provider(String),
    Tool { tool_name: String, message: String },
    ContextLimit { tokens: u64 },   // hits when context full — no graceful recovery
    SessionNotFound { session_id: String },
    PermissionDenied { tool_name: String },
    BudgetExceeded,
    MaxTurnsExceeded,
    Interrupted,
    // ...
}
```

- Provider errors: exponential backoff per provider (Anthropic: 6 attempts, 5 server, 4 network)
- Tool errors: returned as error `ToolResult` to agent (agent decides to retry or escalate)
- Budget exceeded: hard stop with clear error
- No doom-loop detection
- No "reject with feedback" to agent

**OpenCode**:

```typescript
// Error types with structured data:
class DeniedError extends Schema.TaggedErrorClass("PermissionDeniedError", {})
class RejectedError extends Schema.TaggedErrorClass("PermissionRejectedError", {})
class CorrectedError extends Schema.TaggedErrorClass("PermissionCorrectedError", {
  feedback: Schema.String  // User's feedback text sent to agent
})
class ContextOverflowError // Auto-triggers compaction

// Doom loop: if 3+ consecutive identical tool failures → asks permission to continue
// Retry: provider-level retry with provider-specific backoff
// Compaction: auto-triggered on overflow, returns "stop" if unable to compact
```

**Delta**: OpenCode has more sophisticated error architecture. `CorrectedError` with feedback is a key missing feature in Clido — it allows the user to redirect the agent ("don't delete that file, copy it instead") without starting over. The doom-loop detection prevents runaway spending.

### 4.10 Performance & Responsiveness

**Clido**:

- **Startup time**: ~10ms (compiled Rust binary, no runtime)
- **First token**: Limited by provider (same for both)
- **Tool execution**: Parallel read-only tools (semaphore, configurable)
- **Context assembly**: O(n) token estimation on every turn
- **Index build**: SQLite FTS5 index, background build
- **No overhead**: No event bus, no database writes per message

**OpenCode**:

- **Startup time**: ~500ms-2s (Bun runtime init + SQLite migration + Effect layer composition)
- **First token**: Same provider latency
- **Tool execution**: Concurrent, bounded by provider
- **Context assembly**: Token estimation + SQLite reads on every turn
- **Database writes**: Every message part written to SQLite in real-time
- **Plugin loading**: Bun installs npm packages per config directory

**Delta**: **Clido is dramatically faster to start and has lower per-turn overhead.** For CI/CD pipelines, scripting, and power users running many short tasks, this matters enormously. OpenCode's Bun runtime adds 500ms+ startup that Clido completely avoids.

### 4.11 Architecture & Code Quality

**Clido**:

```
Strengths:
- Single-responsibility crates with clear boundaries
- Trait-based tool interface (Tool, ModelProvider, AskUser, EventEmitter)
- Zero-cost abstractions (Rust ownership, no GC pressure)
- Comprehensive unit tests in every crate
- Audit log with secret redaction
- Well-documented (//! module-level docs on every file)

Weaknesses:
- Agent loop doesn't use streaming — batches complete responses
- JSONL session storage doesn't support efficient queries
- MCP client uses blocking I/O (Mutex<BufReader>) — will block on slow MCP servers
- Planner parallel batches not yet parallel
- Context compaction is simple truncation (lossy)
```

**OpenCode**:

```
Strengths:
- Effect framework enables composable dependency injection
- SQLite gives efficient session queries and ACID guarantees
- Bus-based decoupling enables multi-surface (CLI, web, desktop, Slack)
- Plugin hook system allows arbitrary extension
- Per-module Effect Services are strongly typed

Weaknesses:
- Effect framework is a steep learning curve (non-idiomatic TypeScript)
- Single-word naming convention (AGENTS.md mandate) hurts readability
- Test coverage is lower than claimed (tests cannot run from repo root)
- Bun runtime lock-in (bun-specific APIs throughout)
- Heavy dependency tree (15+ AI SDK packages)
- 285 TypeScript files with complex cross-cutting concerns
- InstanceState pattern creates implicit global state per workspace
```

### 4.12 Testing & Reliability

**Clido**:

- Unit tests in every crate: 50+ tests in `clido-agent`, `clido-tools`, `clido-planner`
- Integration tests: `v3_integration.rs`, `integration.rs`
- Benchmark: `benches/startup.rs`
- Tests run with `cargo test` from any directory
- CI/CD friendly (no special runtime requirements)
- `concurrent_correctness.rs` — concurrency tests for agent loop

**OpenCode**:

- Tests explicitly cannot run from repo root (`guard: do-not-run-tests-from-root`)
- Must run from `packages/opencode`
- Avoid mocks — tests against real implementation
- e2e tests in `packages/app/e2e`
- Type checking via `bun typecheck` (not `tsc`)
- Lower unit test density than Clido

**Delta**: **Clido's testing setup is more developer-friendly and reliable.** OpenCode's restriction on running tests from root is a significant friction point.

---

## 5. Best Practices to Adopt from OpenCode

### 5.1 Two-Strategy Context Compaction (HIGH PRIORITY)

**What**: OpenCode uses pruning (remove old tool outputs) + summary compaction (LLM generates structured summary).

**Why better**: Clido's truncation loses information. Agents on long tasks forget earlier decisions, fail to reference code they already read, re-examine files they already know. This causes wasted turns and incorrect behavior.

**How to adopt in Clido**:

```rust
// Step 1: Pruning (in clido-context)
pub async fn prune_old_tool_outputs(
    history: &mut Vec<Message>,
    protect_last_n_tokens: u32,  // 40_000
    min_freed_tokens: u32,       // 20_000
) -> u32 { /* iterate backward, zero tool results older than protect boundary */ }

// Step 2: Summary compaction (in clido-agent)
pub async fn compact_to_summary(
    provider: &dyn ModelProvider,
    history: &[Message],
    config: &AgentConfig,
) -> Result<Message> {
    // Use same/cheaper model to generate summary
    // Structured prompt: Goal / Instructions / Discoveries / Accomplished / Files
    // Returns as a synthetic user message with compaction marker
}
```

### 5.2 Permission Ruleset with Per-File Patterns (HIGH PRIORITY)

**What**: OpenCode uses `[{permission, pattern, action}]` arrays evaluated via last-wins wildcard matching.

**Why better**: Clido's binary modes can't distinguish "always allow edits to tests" from "always ask for production configs". OpenCode allows e.g. `{ edit: { "*.sql": "ask", "*": "allow" } }`.

**How to adopt in Clido**:

```rust
// New config option in clido-core/src/config.rs
pub struct PermissionRuleset {
    pub rules: Vec<PermissionRule>,
}

pub struct PermissionRule {
    pub permission: String,    // "edit", "write", "bash", "read", "*"
    pub pattern: String,       // glob: "*.sql", "src/**/*.ts", "*"
    pub action: PermAction,    // Allow, Deny, Ask
}

// Extend PermissionMode to support rules-based mode
pub enum PermissionMode {
    Default,       // ask all write ops
    AcceptAll,     // allow all
    PlanOnly,      // deny all write
    DiffReview,    // show diff before write ← keep this unique feature
    RuleSet(PermissionRuleset),  // NEW: rule-based
}
```

Keep `DiffReview` as a unique Clido capability — OpenCode has no equivalent.

### 5.3 Reject-With-Feedback to Agent (MEDIUM PRIORITY)

**What**: OpenCode's `CorrectedError` sends user feedback text back to the agent as an error message.

**Why better**: Instead of the agent failing and stopping, the user can redirect ("don't delete the file, just comment it out") and the agent incorporates this feedback in its next step.

**How to adopt**:

```rust
pub enum PermGrant {
    Allow,
    Deny,
    EditInEditor,
    AllowAll,
    DenyWithFeedback(String),  // NEW: deny + reason for agent
}

// In execute_tool_maybe_gated:
PermGrant::DenyWithFeedback(feedback) => {
    ToolOutput::error(format!(
        "Tool call rejected by user. Feedback: {}. Adjust your approach accordingly.",
        feedback
    ))
}
```

### 5.4 Doom-Loop Detection (MEDIUM PRIORITY)

**What**: OpenCode detects when the agent makes 3+ identical failing tool calls in a row.

**Why better**: Without this, a confused agent can burn through budget making the same failing call repeatedly.

**How to adopt**:

```rust
// In agent_loop.rs, track recent tool results
struct LoopMonitor {
    recent_failures: VecDeque<(String, String)>,  // (tool_name, truncated_error)
    doom_threshold: u32,  // default 3
}

impl LoopMonitor {
    fn check_doom_loop(&mut self, tool_name: &str, error: &str) -> bool {
        let key = format!("{}:{}", tool_name, &error[..error.len().min(100)]);
        self.recent_failures.push_back((tool_name.to_string(), key.clone()));
        if self.recent_failures.len() > self.doom_threshold as usize {
            self.recent_failures.pop_front();
        }
        // All recent failures are the same → doom loop
        self.recent_failures.len() >= self.doom_threshold as usize &&
        self.recent_failures.iter().all(|(_, k)| k == &key)
    }
}
```

### 5.5 `@agent` Mention Routing (MEDIUM PRIORITY)

**What**: `@explore my codebase`, `@general do X and Y in parallel` — routes directly to named sub-agents.

**Why better**: Power users can direct work to specialized agents without changing modes or restart. The `@explore` agent saves tokens by using a read-only, lower-cost model.

**How to adopt**:

```rust
// In REPL prompt parsing:
fn parse_mentions(prompt: &str) -> (String, Option<String>) {
    // "@explore find all API endpoints" → (agent: "explore", prompt: "find all API endpoints")
    // "@general build X and test Y" → (agent: "general", prompt: ...)
    // "fix the bug" → (agent: None, prompt: "fix the bug")
}
```

### 5.6 Session-Level Permission Overrides (LOW PRIORITY)

**What**: Each session can carry a `Permission.Ruleset` that overrides project-level rules for sub-agent sessions.

**Why better**: Sub-agents launched from `SpawnWorkerTool` can be sandboxed to specific paths/tools without changing global config.

**How to adopt**:

```rust
// In AgentSetup / AgentConfig:
pub struct AgentConfig {
    // ...
    pub session_permission_overrides: Option<PermissionRuleset>, // NEW
}
```

### 5.7 LiteLLM Proxy Compatibility (LOW PRIORITY)

**What**: OpenCode auto-detects LiteLLM proxies and adds a dummy `_noop` tool when tool call history requires a tools parameter.

**Why better**: Prevents cryptic "tools field required" errors from LiteLLM proxies when returning to a conversation that has prior tool calls.

**How to adopt**: In the OpenAI-compatible provider, detect when `tools` array would be empty but `messages` contains prior `tool_use` content, and add a stub `_noop` tool.

---

## 6. Clido Strengths

### 6.1 DiffReview Permission Mode (UNIQUE)

```rust
PermissionMode::DiffReview  // Show unified diff before EVERY write/edit
```

This is entirely absent from OpenCode. OpenCode shows diffs *after* edits in the UI but doesn't let you interactively approve/deny individual edits at the diff level with editor integration. The `EditInEditor` grant in Clido (open proposed content in `$EDITOR`) is particularly powerful for surgical adjustments.

**Strategic move**: Promote this feature heavily. Add `--diff-review` shorthand. Make the diff output rich (syntax highlighted). Add a "reject with feedback" option to the diff review dialog. This is a genuine differentiator for cautious users who want to stay in control without going to `PlanOnly`.

### 6.2 Structured DAG Planner (UNIQUE)

```rust
// clido-planner: TaskGraph with validated dependencies
pub struct TaskNode {
    pub depends_on: Vec<TaskId>,      // explicit dependency edges
    pub tools: Option<Vec<String>>,   // tool allowlist per task
    pub complexity: Complexity,       // Low/Medium/High metadata
}
```

OpenCode's plan mode is freeform markdown. Clido's DAG planner has:

- Validated topological sort (no cycles, no missing deps)
- Complexity metadata per task
- Tool allowlist per task (sub-agents get exactly the tools they need)
- JSON serialization (plans are machine-readable artifacts)

**Strategic move**: Enable true parallel execution in `PlanExecutor`. Make plan visualization available (JSON → mermaid diagram). Add `clido plan show` to display the current plan as a tree. Store plans across sessions.

### 6.3 YAML Declarative Workflows (UNIQUE)

OpenCode has no equivalent to `clido-workflows`. Declarative YAML workflows with steps, retries, backoff, templates, and prerequisite checking are a powerful automation primitive for teams.

```yaml
# Example .clido/workflows/daily-review.yaml
name: daily-code-review
steps:
  - id: fetch_changes
    tool: Git
    args: { subcommand: "diff --stat HEAD~1" }
  - id: review
    depends_on: [fetch_changes]
    tool: Bash
    args: { command: "clido 'Review these changes: {{steps.fetch_changes.output}}'" }
    retry:
      max_attempts: 2
      backoff: exponential
```

**Strategic move**: Add workflow templates for common dev tasks (PR review, release notes, test generation). Make workflows composable. Enable workflow sharing.

### 6.4 Sandbox Execution (UNIQUE)

```rust
// macOS: sandbox-exec with restrictive profile (network deny, fs write limited to /tmp)
// Linux: bwrap (--tmpfs /tmp, --unshare-net, --die-with-parent)
```

OpenCode has no sandboxing. Clido's bash tool can optionally run in a proper OS-level sandbox. This is critical for security-conscious users and enterprise deployments.

**Strategic move**: Make sandboxing the default for new installations (opt-out rather than opt-in). Document the sandbox restrictions clearly. Add `--sandbox` flag. Test with common developer tools to ensure they work within the sandbox.

### 6.5 Secret Environment Stripping (UNIQUE)

```rust
const SECRET_ENV_VARS: &[&str] = &[
    "ANTHROPIC_API_KEY", "OPENAI_API_KEY", "OPENROUTER_API_KEY",
    "AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", /* ... 17 vars */
];
fn strip_secret_env_vars(cmd: &mut tokio::process::Command) { /* removes from child env */ }
```

OpenCode doesn't strip secret env vars before spawning bash commands. Clido prevents accidental exposure of API keys through command output or `env` / `printenv` leaks.

### 6.6 Budget Controls (UNIQUE among CLI tools)

```rust
pub max_budget_usd: Option<f64>,  // Hard stop when cumulative cost exceeds limit
```

Combined with the pricing table and per-turn cost display, Clido gives users precise spending control. OpenCode tracks costs but has no hard budget limit.

**Strategic move**: Add budget warnings at 50%/80%/90% of limit. Add `--budget $1.00` shorthand. Display remaining budget in REPL prompt.

### 6.7 Audit Log with Secret Redaction

```rust
// Writes JSONL audit log for every tool call:
// { tool_name, input_summary, output_hash, duration_ms, timestamp }
// Input values containing secret patterns are replaced with [REDACTED]
```

Critical for enterprise use. OpenCode has no audit trail.

**Strategic move**: Add `clido audit` subcommand to review audit log. Add structured audit events for permission decisions. Support log shipping to external SIEM systems.

### 6.8 Named Role System

```toml
[roles]
fast = "claude-3-5-haiku-20241022"
reasoning = "claude-opus-4-5"
critic = "gpt-4o"
planner = "claude-3-5-sonnet"
```

This lets users define semantic model aliases (`@fast`, `@reasoning`) rather than hardcoding model IDs. When models are updated, only the config changes. OpenCode has model-per-agent config but no named role abstraction.

### 6.9 CI/CD & Scripting Excellence

```sh
# Non-interactive, JSON output, env-driven
CLIDO_PERMISSION_MODE=accept-all \
CLIDO_MAX_BUDGET_USD=0.50 \
CLIDO_OUTPUT_FORMAT=json \
clido -p "generate release notes for v1.2.0"
```

Clido's design is inherently scriptable. 10ms startup time, JSON output, full env var control, `-p` non-interactive mode — all make it ideal for CI pipelines. OpenCode's 500ms+ Bun startup and TUI-first design make it less suitable for automation.

---

## 7. Missing Features

### Critical Missing Features (Block Competitiveness)


| Feature                           | Impact | Effort | Notes                                          |
| --------------------------------- | ------ | ------ | ---------------------------------------------- |
| **Summary-based compaction**      | High   | Medium | Current truncation loses context on long tasks |
| **Full-screen TUI**               | High   | High   | Largest visible UX gap                         |
| **Per-file permission rules**     | High   | Medium | Needed for real-world projects                 |
| `**ls` (directory listing) tool** | Medium | Low    | Basic filesystem tool that's missing           |
| **apply_patch tool**              | Medium | Medium | GPT-4 models work better with patch format     |
| **Real-time streaming in output** | Medium | Medium | Users can't see agent "thinking"               |
| **Reject-with-feedback**          | Medium | Low    | Critical for interactive refinement            |
| **Doom loop detection**           | Medium | Low    | Prevents runaway spending                      |


### Important Missing Features (Significant Gap)


| Feature                         | Impact | Effort | Notes                                     |
| ------------------------------- | ------ | ------ | ----------------------------------------- |
| **LSP integration**             | Medium | High   | Go-to-def, refs, hover available to agent |
| **@agent routing syntax**       | Medium | Low    | Route to specialized agents mid-session   |
| **Session sharing/public URLs** | Medium | High   | Collaboration feature                     |
| **Snapshot/revert per message** | Medium | High   | Git-based state recovery                  |
| **Plugin/extension system**     | Medium | High   | Ecosystem moat                            |
| **Todo tracking tool + UI**     | Low    | Medium | Visual task tracking                      |
| **Model switching mid-session** | Low    | Medium | No restart needed                         |
| **Worktree isolation**          | Low    | High   | Parallel sessions without conflicts       |


### Nice-to-Have Features


| Feature                         | Notes                   |
| ------------------------------- | ----------------------- |
| **Session forking**             | Branch from any message |
| **Session export/import**       | Portability             |
| **Storybook for UI components** | OpenCode has this       |
| **Slack integration**           | Team notifications      |
| **Desktop app**                 | Electron/Tauri          |
| **OAuth auth providers**        | SSO for teams           |


---

## 8. Critical Weaknesses

### W1: Context Compaction (Severity: HIGH)

**Current behavior**: When `estimate_tokens_messages(history) > max_context_tokens * threshold`, Clido truncates the oldest messages (sliding window). No summary is generated. The agent loses awareness of all decisions, files read, and context established earlier in the conversation.

**Impact**: On tasks longer than ~15-20 turns, the agent begins re-reading files it already knows, making decisions inconsistent with earlier choices, and confusing the user by appearing to forget previous instructions.

**Fix**: Implement the two-strategy approach: (1) prune old tool outputs first (low-information-loss), (2) use summary compaction when still over limit.

### W2: No Full-Screen TUI (Severity: HIGH)

**Current behavior**: Plain REPL to stderr. Spinner shows current tool name. Results printed to stdout.

**Impact**: The user experience feels dated compared to OpenCode's live-streaming TUI. Users cannot see agent reasoning in real-time, cannot expand tool call details, cannot see the full conversation history without scrolling, and cannot manage sessions visually.

**Fix**: Build a full-screen TUI using `ratatui` (Rust TUI library). Key components: conversation panel with streaming text, tool call list with expand/collapse, session sidebar, status bar with model/cost/tokens.

### W3: Binary Permission Model (Severity: HIGH)

**Current behavior**: Four modes apply globally. No ability to say "ask for this file type, allow for others."

**Impact**: In real projects, users either grant too much (`AcceptAll`) or get interrupted too frequently (`Default`). The lack of per-file rules makes Clido frustrating for large codebases with mixed sensitivity.

**Fix**: Add rule-based permission mode (see §5.2).

### W4: Provider Coverage Gap (Severity: MEDIUM)

**Current behavior**: 7 providers supported. Missing: Azure, Gemini, Vertex, Bedrock, GitLab, GitHub Copilot, Groq, xAI.

**Impact**: Enterprise users on Azure OpenAI or teams using GitHub Copilot cannot use Clido. Clido misses the entire Google AI ecosystem.

**Fix**: Add Vercel AI SDK or equivalent for Rust. The `clido-providers` crate is already trait-based — adding new providers is straightforward. Priority: Azure (enterprise), Gemini (free tier users), GitHub Copilot (existing subscribers).

### W5: Synchronous Agent Loop (Severity: MEDIUM)

**Current behavior**: `AgentLoop::run()` uses `complete()` (non-streaming). The user sees no output until the model finishes its entire response.

**Impact**: On slow or expensive models (claude-opus-4-5), users wait 30-60 seconds per turn with no visible progress beyond a spinner. This feels broken.

**Fix**: Switch the agent loop to use `complete_stream()`. Emit text deltas via `EventEmitter::on_assistant_text()` so the REPL (or TUI) can stream output in real-time.

### W6: MCP Client Blocking I/O (Severity: MEDIUM)

**Current behavior**: `McpClient` uses `Mutex<BufReader<ChildStdout>>` — synchronous blocking reads.

**Impact**: A slow or misbehaving MCP server will block the entire Tokio executor thread. This can stall the entire agent loop.

**Fix**: Rewrite `McpClient` to use async tokio I/O (`tokio::io::BufReader`, `tokio::io::AsyncBufReadExt`).

### W7: DAG Planner Not Parallel (Severity: MEDIUM)

**Current behavior**: `PlanExecutor::execute()` processes parallel batches sequentially. The code comment confirms this: "Run batch tasks sequentially for now."

**Impact**: The `--planner` flag adds latency (plan generation step) without delivering the parallelism benefit. A 5-task plan with no dependencies takes 5x longer than necessary.

**Fix**: In `PlanExecutor::execute()`, replace the inner `for task in batch` with `join_all(batch.iter().map(|task| runner.run_task(task, &context)))`. Note: context passing needs to be read-only for parallel tasks.

### W8: Session Persistence Fragility (Severity: LOW-MEDIUM)

**Current behavior**: Sessions stored as JSONL files. Resume reconstructs history by replaying all lines.

**Impact**: JSONL files are append-only and not queryable. No efficient "search sessions by content". Corruption of one line can break entire session replay. File-per-session doesn't scale well to thousands of sessions.

**Fix**: Migrate to SQLite for session storage. Keep JSONL as export format. This enables efficient session search, metadata queries, and reliable atomic writes.

---

## 9. Improvement Roadmap

### Phase 1: Core Quality (1-3 months) — Close Critical Gaps


| Item                              | Priority | Description                                                                                 |
| --------------------------------- | -------- | ------------------------------------------------------------------------------------------- |
| **1.1 Summary compaction**        | P0       | Implement LLM-based context summarization on overflow. Keep pruning as first-pass strategy. |
| **1.2 Streaming agent loop**      | P0       | Switch `run()` to use `complete_stream()`. Emit deltas via `on_assistant_text()`.           |
| **1.3 Doom loop detection**       | P1       | 3+ identical tool failures → ask user to continue or stop.                                  |
| **1.4 Reject-with-feedback**      | P1       | New `PermGrant::DenyWithFeedback(String)` variant.                                          |
| **1.5 `ls` tool**                 | P1       | Directory listing with depth limit. Tiny effort, fills basic gap.                           |
| **1.6 Async MCP client**          | P1       | Replace blocking I/O in `McpClient` with tokio async I/O.                                   |
| **1.7 Per-file permission rules** | P1       | `PermissionMode::RuleSet(PermissionRuleset)` with wildcard matching.                        |


### Phase 2: UX & Visibility (3-6 months) — Close Visible Gaps


| Item                                | Priority | Description                                                                                   |
| ----------------------------------- | -------- | --------------------------------------------------------------------------------------------- |
| **2.1 Full-screen TUI**             | P0       | `ratatui`-based TUI. Panels: chat (streaming), tool calls (expandable), sessions, status bar. |
| **2.2 @agent routing syntax**       | P1       | `@explore`, `@worker`, custom agents — parsed from prompt.                                    |
| **2.3 Enable DAG parallelism**      | P1       | `join_all()` in `PlanExecutor`. Huge improvement to `--planner` value.                        |
| **2.4 apply_patch tool**            | P1       | Unified diff format for GPT-4 models. Reuse OpenCode's patent approach.                       |
| **2.5 Model switching mid-session** | P2       | `/model claude-opus-4-5` slash command. `AgentLoop::set_model()` already exists.              |
| **2.6 Budget warnings**             | P2       | 50%/80%/90% budget threshold warnings with remaining display.                                 |


### Phase 3: Platform Expansion (6-12 months) — Build Moat


| Item                           | Priority | Description                                                                   |
| ------------------------------ | -------- | ----------------------------------------------------------------------------- |
| **3.1 Provider expansion**     | P1       | Add Azure, Gemini, Groq, xAI, GitHub Copilot.                                 |
| **3.2 SQLite session storage** | P1       | Replace JSONL with SQLite. Efficient queries, atomic writes.                  |
| **3.3 LSP integration**        | P2       | Language server tools (definition, references, hover). Use `lsp-types` crate. |
| **3.4 Snapshot/revert**        | P2       | Git-based session state snapshots. `clido revert` command.                    |
| **3.5 Plugin system**          | P2       | Dynamic tool loading from WASM or subprocess.                                 |
| **3.6 Skills system**          | P2       | CLIDO-SKILL.md files for reusable agent instructions.                         |
| **3.7 Session sharing**        | P3       | Public share URLs for sessions (opt-in).                                      |
| **3.8 Worktree isolation**     | P3       | Per-session git worktrees for parallel work.                                  |


### Phase 4: Differentiation (12+ months) — Build Unique Value


| Item                             | Priority | Description                                                  |
| -------------------------------- | -------- | ------------------------------------------------------------ |
| **4.1 Parallel DAG execution**   | P1       | True multi-provider parallel task execution via `--planner`. |
| **4.2 Workflow marketplace**     | P2       | Share/discover YAML workflows.                               |
| **4.3 Enterprise audit & SSO**   | P2       | Audit log streaming to SIEM. SAML/OIDC authentication.       |
| **4.4 Clido-as-API**             | P2       | REST/gRPC server mode for integration.                       |
| **4.5 Reinforcement from audit** | P3       | Use audit log data to improve permission rule suggestions.   |


---

## 10. Path to Superiority

### Where Clido Can Definitively Win

**1. CI/CD & Automation — The Scriptable Agent**

OpenCode is TUI-first and Bun-dependent. Clido's 10ms startup, JSON output, full env-var control, and Rust portability make it the natural choice for automation. Lean into this: add OpenAPI spec for the JSON output format, publish container images, add GitHub Actions integration.

```yaml
# .github/workflows/code-review.yml
- uses: clido/action@v1
  with:
    prompt: "Review this PR for security issues"
    budget: "$0.25"
    output: json
    permission-mode: accept-all
```

**2. Enterprise Security — The Trustworthy Agent**

OpenCode has no sandboxing, no audit trail, no secret env stripping, no budget limits. Clido has all four. Package these as the "enterprise security bundle": sandbox execution, audit log with redaction, budget controls, and per-file permission rules (once added). Target security-conscious teams where running an AI agent on production code requires justification.

**3. The Precision Edit Experience**

The `DiffReview` mode + `EditInEditor` grant + reject-with-feedback creates a precision editing experience that OpenCode cannot match. Rename this to `--interactive` or `--precise` mode, make it discoverable, and build a diff review TUI component that highlights specific changes. This targets developers who want AI assistance but retain full control over every change.

**4. Smart Multi-Agent Orchestration**

Clido's worker/reviewer architecture with the DAG planner is uniquely powerful once parallel execution is enabled. Position: "The only CLI agent that can decompose and parallelize complex tasks with explicit dependency tracking." Build on this with workflow templates for common patterns (PR review, refactor + test + document).

**5. The YAML Workflow Engine**

No other major CLI agent has a declarative workflow engine. Build a workflow library. Enable `clido workflow run daily-review`. Create a community workflow registry. This is the highest-leverage differentiation for team use.

### Specific Technical Wins Available Now

1. **Turn on DAG parallel execution** (one afternoon of work) — transforms `--planner` from overhead to advantage
2. **Add summary compaction** (one week) — closes the most impactful quality gap
3. **Stream agent output** (two days) — eliminates the most jarring UX gap (silent waiting)
4. **Add `ls` tool** (one hour) — fills a basic gap with trivial effort
5. **Add doom-loop detection** (half day) — budget protection and professionalism

---

## 11. Final Critical Reflection

### What This Analysis Got Right in Pass 1 That Holds

- Clido's CLI-first, automation-friendly design is a genuine competitive moat
- OpenCode's permission model is architecturally superior
- The TUI gap is real and significant
- Clido's safety features (sandbox, audit, budget) are undervalued and under-promoted

### What Earlier Analysis Missed (Discovered in Passes 3-5)

**Pass 3 Correction**: The initial assessment underestimated how severely Clido's simple truncation compaction will hurt real-world performance. On tasks over 20 turns (common for non-trivial coding tasks), agents routinely re-read files, forget earlier decisions, and become confused. This is not a "nice to have" — it's a correctness issue.

**Pass 4 Correction**: OpenCode's Effect framework is not just a stylistic choice — it provides real correctness guarantees (typed error channels, scoped resource management) that Clido's Result/anyhow approach can't match. But the complexity cost is real: the codebase is harder to understand, debug, and contribute to. Clido's architecture is more approachable, which matters for long-term maintainability and community contribution.

**Pass 5 Correction**: The MCP blocking I/O issue in Clido is more serious than initially assessed. With multiple MCP servers configured, a single slow server will stall the entire agent loop — this can cause apparent hangs of 30+ seconds in real use.

### Where OpenCode Has Technical Debt Clido Should Not Copy

1. **Single-word naming mandate** (AGENTS.md): Optimizes for fewer keystrokes at the cost of readability. `pid` is fine; forcing `const __ = await Flock.acquire(...)` obscures meaning.
2. **Effect for everything**: The Effect framework is powerful but creates massive cognitive overhead. New contributors need to learn Effect, Effect-TS ecosystem, and OpenCode-specific patterns simultaneously.
3. **Bun lock-in**: Using `Bun.file()`, `Bun.write()`, and bun-specific APIs throughout makes the codebase non-portable. Clido's standard Rust stdlib + tokio are universally portable.
4. **Tests cannot run from root**: This is a real quality-of-life issue for contributors. Clido's `cargo test --workspace` from anywhere is strictly better.

### The Core Strategic Insight

OpenCode built a platform. Clido built a tool.

Platforms win by ecosystem (plugins, integrations, community). Tools win by being the best at their specific job. Clido should not try to out-platform OpenCode. Instead, Clido should be demonstrably the best agent for:

- Automation, scripting, and CI/CD pipelines
- Security-conscious environments  
- Precise, human-supervised editing
- Multi-model orchestration with explicit task graphs

These are four clear use cases where Clido's architecture is already better-suited and where the remaining gaps are closeable in months, not years.

The highest-ROI single investment: **build the full-screen TUI** (Phase 2.1). It won't change Clido's underlying architecture, but it closes the largest visible gap and makes all the existing capabilities discoverable to users who evaluate tools by first impressions.

---

*Analysis by GitHub Copilot — 2026-03-27. Based on full source examination of both codebases via 5-pass reverse engineering methodology.*