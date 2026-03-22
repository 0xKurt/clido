# Feature Plan: Mid-Session Model Switching

**Status:** Planned
**Target release:** V2
**Crate(s) affected:** `clido-cli`, `clido-agent`, `clido-core`
**New files:** `crates/clido-agent/src/routing.rs`, `docs/guide/model-switching.md`

---

## 1. Problem Statement

Clido locks the model at session start via `AgentConfig::model` and never changes it. This is a reasonable default but a real limitation for multi-step workflows.

A typical session might start with: "Find all places in the codebase where we use the old auth library" (a search task — cheap, fast model appropriate), then "Now refactor all of them to use the new library" (a complex multi-file edit — expensive, powerful model appropriate), then "Summarize what changed" (cheap again). Today this requires the user to pick one model for the entire session, either overpaying on search tasks or under-powering on refactor tasks.

Claude Code does not support mid-session switching at all (model is fixed per chat window). Cursor supports selecting a model per-message in its composer UI. Cline has a configuration option for switching models but it requires editing `settings.json` and restarting. None of these tools support automatic model routing based on task complexity.

The absence of mid-session switching makes Clido significantly more expensive to use effectively — users pay powerful-model prices for every turn, including the trivial ones.

---

## 2. Competitive Analysis

| Tool | Session switch | Per-turn switch | Named shortcuts | Auto-routing | Cost tracked across switches | Inline @ syntax |
|------|---------------|-----------------|-----------------|--------------|------------------------------|-----------------|
| Claude Code | No | No | No | No | N/A | No |
| Cursor | No (per window) | Yes (composer UI) | No | No | No | No |
| Cline | Config restart | No | No | No | No | No |
| Aider | `/model` command | No | No | No | No | No |
| **Clido (this plan)** | **Yes (`/model`)** | **Yes (`@model`)** | **Yes (`/fast`, `/smart`)** | **Yes (heuristic)** | **Yes (cumulative)** | **Yes** |

Clido is the only terminal coding agent in this comparison to support all five switching modes simultaneously. The auto-routing heuristic is unique across all compared tools.

---

## 3. Our Specific Improvements

### 3.1 Three switching modes
Clido supports three distinct model-switching modes, each suited to a different workflow:

**Mode 1 — Session switch (`/model <name>`):** All subsequent turns in the session use the new model. Conversation history is fully preserved and included in the new model's context. Cost accumulates correctly across models.

**Mode 2 — Per-turn switch (`@model-name <prompt>`):** A single turn is executed with the named model. After that turn completes, the session reverts to the previous model automatically. This is for one-off "ask the powerful model about this one thing" moments without committing to a full session switch.

**Mode 3 — Auto-routing (`auto_route = true` in config):** Before each turn, a lightweight complexity heuristic (§3.4) decides whether to use the configured `fast_model` or `smart_model`. The user never has to think about model selection; Clido makes the choice automatically.

### 3.2 Named shortcuts `/fast` and `/smart`
Rather than requiring users to memorize model IDs, `/fast` switches to the cheapest model listed in `pricing.toml` that supports the current provider, and `/smart` switches to the most capable model. These are resolved dynamically from `pricing.toml`, so they automatically pick the best option as new models are added.

Users can also configure explicit overrides:
```toml
[agent]
fast_model = "claude-haiku-4-5"
smart_model = "claude-opus-4"
```

### 3.3 Inline `@model-name` syntax
Typing `@claude-opus-4 what is the best architecture for this?` at the TUI input sends that turn with `claude-opus-4` regardless of the current session model, then reverts. This is parsed at the input layer before the message is dispatched to the agent loop. No new command syntax is needed — it blends naturally into the prompt.

### 3.4 Auto-routing heuristic
When `auto_route = true`, each turn is scored before sending. The heuristic runs in `clido-agent/src/routing.rs`:

**Inputs:**
- Prompt word count
- Number of tool calls in the last 3 turns
- Presence of complexity-indicating keywords

**Scoring:**

| Signal | Weight | Threshold |
|--------|--------|-----------|
| Prompt word count > 50 | +1 | complex |
| Prompt word count > 150 | +1 | complex |
| Recent tool calls > 3 | +1 | complex |
| Recent tool calls > 6 | +1 | complex |
| Keyword: refactor, architect, design, rewrite, migrate | +2 | complex |
| Keyword: search, find, show, list, explain, summarize | -1 | simple |

Score >= 3 → `smart_model`. Score < 3 → `fast_model`.

The routing decision is shown in the TUI as a dim info line: `  [auto: using claude-opus-4 (complexity score: 4)]`.

### 3.5 Full history preserved across all switch modes
The conversation history `Vec<Turn>` on the `AgentLoop` is never cleared on model switch. All prior turns are sent as context to the new model on the next request. This means the new model has full awareness of what was discussed with the previous model.

### 3.6 Per-model cost tracking
`AgentLoop` tracks a `HashMap<String, f64>` from model ID to cumulative cost. The session footer in TUI shows total cost (all models combined). The `/cost` slash command shows a breakdown per model: `claude-opus-4: $0.0412 | claude-haiku-4-5: $0.0018 | total: $0.0430`.

### 3.7 Provider validation on switch
Switching to a model from a different provider (e.g., from Anthropic to OpenAI) requires the target provider to be configured in `config.toml`. If the `[openai]` section is absent or has no API key, the switch fails immediately with:
```
Error: Model "gpt-4o" requires provider "openai" which is not configured.
Add [openai] api_key = "..." to ~/.config/clido/config.toml
```

---

## 4. Design Decisions and Rationale

**Model switch state lives on `AgentLoop`, not on `AgentConfig`.**
`AgentConfig` is immutable after session start. Adding a mutable `current_model: String` field to `AgentLoop` keeps the config pure and makes the switch state explicit and testable. `AgentConfig::model` becomes the "initial model" or "base model" to revert to after per-turn switches.

**Auto-routing heuristic is fast and deterministic, not LLM-based.**
An LLM-based router (like a small model deciding which big model to use) adds latency, cost, and a failure mode. A keyword + word-count heuristic runs in microseconds, is auditable, and produces predictable results. Users can inspect the scoring with `CLIDO_LOG=debug`.

**Per-turn revert uses a stack, not a flag.**
`model_stack: Vec<String>` on `AgentLoop`. A `/model` or `/fast`/`/smart` switch pushes the new model. A per-turn `@model` push creates a one-entry stack frame that is popped after the turn. This design generalizes: future nested switching (e.g., a tool that spawns a sub-agent with a different model) works correctly.

**`/fast` and `/smart` are slash commands, not just config aliases.**
Making them first-class slash commands means they appear in `/help`, are tab-completable, and are documented consistently with other commands. They also allow the TUI to show "switched to fast model (claude-haiku-4-5)" rather than just a raw model name change.

**Routing decision logged as `AgentEvent`, not just to stderr.**
Emitting an `AgentEvent::ModelSwitch` lets the TUI display the routing decision inline in the transcript, making the auto-routing behavior transparent and debuggable without polluting the main output stream.

---

## 5. Implementation Steps

### 5.1 New module: `crates/clido-agent/src/routing.rs`

```rust
pub struct RoutingContext {
    pub prompt: String,
    pub recent_tool_call_count: usize,  // sum of tool calls in last 3 turns
}

pub struct RoutingDecision {
    pub model: String,
    pub score: i32,
    pub reason: String,   // human-readable explanation for TUI display
}

pub fn score_complexity(ctx: &RoutingContext) -> i32

pub fn route(
    ctx: &RoutingContext,
    fast_model: &str,
    smart_model: &str,
) -> RoutingDecision
```

Constants for keywords live in `routing.rs` as `const COMPLEX_KEYWORDS` and `const SIMPLE_KEYWORDS` arrays.

### 5.2 `AgentLoop` changes in `agent_loop.rs`

New fields on `AgentLoop`:

```rust
pub model_stack: Vec<String>,           // current model is model_stack.last()
pub per_model_cost: HashMap<String, f64>,
pub auto_route: bool,
pub fast_model: Option<String>,
pub smart_model: Option<String>,
```

New methods:

```rust
pub fn current_model(&self) -> &str
pub fn push_model(&mut self, model: String)
pub fn pop_model(&mut self)   // for per-turn revert
pub fn switch_model(&mut self, model: String, event_tx: &Sender<AgentEvent>)
  // pushes model, emits AgentEvent::ModelSwitch
```

In `run_next_turn()`, before building the API request:
- If `auto_route = true` and no per-turn override is active, call `routing::route()` and push the result.
- After turn completes, pop any per-turn model frame.

### 5.3 Per-turn `@model-name` parsing in TUI input handler (`tui.rs`)

In the input submission handler, check if the input starts with `@`:

```rust
fn parse_per_turn_model(input: &str) -> Option<(String, String)> {
    // returns Option<(model_id, remaining_prompt)>
}
```

If a per-turn model is detected, set `pending_per_turn_model` on TUI state. When the message is dispatched, send `AgentEvent::PerTurnModel { model, prompt }` to the agent loop, which pushes the model for that turn only.

### 5.4 `/model`, `/fast`, `/smart` slash commands in `tui.rs`

Update `SLASH_COMMANDS`:

```rust
("/model",  "Show or switch the current model. Usage: /model [model-name]"),
("/fast",   "Switch to the configured fast (cheap) model for this session."),
("/smart",  "Switch to the configured smart (powerful) model for this session."),
```

`/model` with no argument: existing behavior (show current model).
`/model <name>`: call `agent_loop.switch_model(name)`. Validate name against `pricing.toml` before sending. On unknown name, show inline error: `Unknown model "foo". Run /list-models to see available models.`

`/fast`: resolve `fast_model` from config or `pricing.toml` cheapest-model query. Call `switch_model()`.
`/smart`: resolve `smart_model` similarly.

### 5.5 Provider validation in `clido-core`

Add `fn validate_model_for_provider(model_id: &str, config: &AppConfig) -> Result<()>` to `config_loader.rs`. Checks whether the model's provider (extracted from model ID prefix or `pricing.toml` metadata) has credentials in config. Returns the friendly error from §3.7 on failure.

### 5.6 New `AgentEvent` variants

Add to the `AgentEvent` enum in `tui.rs`:

```rust
ModelSwitch {
    from_model: String,
    to_model: String,
    reason: ModelSwitchReason,
},
```

```rust
pub enum ModelSwitchReason {
    UserCommand,          // /model, /fast, /smart
    PerTurn,              // @model prefix
    AutoRoute { score: i32, rationale: String },
}
```

TUI renders `ModelSwitch` as a dim info line:
- User command: `  → switched to claude-haiku-4-5`
- Auto-route: `  → [auto] claude-opus-4 (score: 4 — complex task)`
- Per-turn: `  → [one turn] claude-opus-4 (will revert after response)`

### 5.7 TUI header update

The TUI header currently shows `[model: claude-sonnet-4-5]`. Update to show the current model from `agent_loop.current_model()`. This must refresh reactively when `AgentEvent::ModelSwitch` is received.

### 5.8 `/cost` command update

Extend the `/cost` slash command handler to show per-model breakdown when more than one model was used in the session:

```
Session cost:
  claude-sonnet-4-5    14 turns   $0.0287
  claude-haiku-4-5      8 turns   $0.0009
  claude-opus-4         2 turns   $0.0521
  ─────────────────────────────────────────
  total                24 turns   $0.0817
```

---

## 6. Config Schema (complete)

```toml
[agent]
# ... existing fields (model, max_turns, max_budget_usd, etc.) ...

# Automatically route each turn to fast or smart model based on complexity.
auto_route = false

# Model to use for low-complexity tasks when auto_route = true.
# When empty, resolved as the cheapest model in pricing.toml for the current provider.
fast_model = ""

# Model to use for high-complexity tasks when auto_route = true, or when /smart is used.
# When empty, resolved as the current session model.
smart_model = ""
```

---

## 7. CLI Surface

The `--model` flag already exists. No new CLI flags are needed; switching is a runtime/TUI feature. Future consideration: `--auto-route` flag for CLI (non-TUI) invocations.

For documentation purposes, the existing flag gains updated help text:

```
--model <MODEL>   Set the starting model for this session.
                  Can be changed mid-session with /model, /fast, or /smart.
                  Use @model-name syntax inline to switch for a single turn.
```

---

## 8. TUI Changes Summary

- Header: shows current model name, updates immediately on switch.
- Input: `@model-name ` prefix detected and highlighted differently from normal text (dim model name, normal prompt).
- New slash commands: `/fast`, `/smart`, extended `/model <name>`.
- `ModelSwitch` event renders as dim info line in transcript.
- Auto-route decision line rendered as dim info before response when `auto_route = true`.
- `/cost` shows per-model breakdown when multiple models used.
- `/help` updated to document `@model` per-turn syntax.

---

## 9. Pricing.toml Model Capability Metadata

Each model entry gains:

```toml
[[models]]
id = "claude-opus-4"
provider = "anthropic"
supports_vision = true
capability_tier = "smart"    # "fast" | "smart" | "balanced"
input_per_mtok = 15.00
output_per_mtok = 75.00

[[models]]
id = "claude-haiku-4-5"
provider = "anthropic"
supports_vision = false
capability_tier = "fast"
input_per_mtok = 0.80
output_per_mtok = 4.00
```

`routing.rs` uses `capability_tier` to resolve `/fast` → cheapest-among-`"fast"` and `/smart` → most-capable-among-`"smart"`.

---

## 10. Test Plan

### Unit tests — `crates/clido-agent/src/routing.rs`

**`test_score_complexity_high_word_count`**
Input: prompt with 200 words, 0 recent tool calls, no keywords. Assert score >= 2.

**`test_score_complexity_many_tool_calls`**
Input: 10-word prompt, 7 recent tool calls. Assert score >= 2.

**`test_score_complexity_complex_keyword`**
Input: prompt containing "refactor", 0 tool calls, 10 words. Assert score >= 2.

**`test_score_complexity_simple_keyword`**
Input: prompt containing only "find", 0 tool calls, 5 words. Assert score <= 1.

**`test_route_returns_smart_for_high_score`**
Mock score returns 4. Assert `route()` returns `smart_model`.

**`test_route_returns_fast_for_low_score`**
Mock score returns 1. Assert `route()` returns `fast_model`.

**`test_route_reason_string_non_empty`**
Assert `RoutingDecision::reason` is non-empty for any input.

### Unit tests — `crates/clido-agent/src/agent_loop.rs`

**`test_push_model_updates_current_model`**
Create `AgentLoop`, call `push_model("claude-haiku-4-5")`. Assert `current_model()` returns `"claude-haiku-4-5"`.

**`test_pop_model_reverts_to_previous`**
Push `"model-a"`, push `"model-b"`, pop. Assert `current_model()` returns `"model-a"`.

**`test_per_turn_model_reverts_after_turn`**
Simulate a per-turn model push and a turn completion. Assert the model stack returns to pre-turn state after `pop_model()` is called in the turn completion path.

**`test_history_preserved_after_model_switch`**
Run two turns with model A. Switch to model B. Run a third turn. Assert the API request for turn 3 includes all prior turns in its messages field (history not cleared on switch).

**`test_per_model_cost_accumulates_correctly`**
Simulate two turns with model A ($0.01 each) and one turn with model B ($0.05). Assert `per_model_cost["model-a"] == 0.02` and `per_model_cost["model-b"] == 0.05`.

### Unit tests — `crates/clido-core`

**`test_provider_validation_passes_when_configured`**
Config has `[anthropic] api_key = "sk-ant-..."`. Model is `claude-opus-4` (provider: anthropic). Assert `validate_model_for_provider()` returns `Ok(())`.

**`test_provider_validation_fails_when_missing`**
Config has no `[openai]` section. Model is `gpt-4o` (provider: openai). Assert `validate_model_for_provider()` returns `Err` whose message mentions `"openai"` and `"not configured"`.

### Unit tests — `crates/clido-cli/src/tui.rs`

**`test_parse_per_turn_model_extracts_model_and_prompt`**
Input: `"@claude-opus-4 explain the auth flow"`. Assert returns `Some(("claude-opus-4", "explain the auth flow"))`.

**`test_parse_per_turn_model_returns_none_for_normal_input`**
Input: `"explain the auth flow"`. Assert returns `None`.

**`test_parse_per_turn_model_returns_none_for_at_in_middle`**
Input: `"email me @ work"`. Assert returns `None` (must start with `@`).

### Integration tests — `crates/clido-cli/tests/model_switching_integration.rs`

**`test_slash_model_command_switches_model_in_session`**
Start a TUI session with model A. Send `/model model-b`. Assert the next API request uses `model-b` in its request payload.

**`test_slash_fast_resolves_to_cheapest_model`**
Config has `fast_model = ""`. Mock `pricing.toml` with two models, one cheaper. Send `/fast`. Assert the cheaper model is selected.

**`test_slash_smart_resolves_to_smart_tier_model`**
Config has `smart_model = ""`. Mock `pricing.toml` with `capability_tier = "smart"` on one model. Send `/smart`. Assert that model is selected.

**`test_auto_route_selects_fast_for_simple_prompt`**
Config has `auto_route = true`, `fast_model = "model-fast"`, `smart_model = "model-smart"`. Send a 5-word prompt with no tools in history. Assert API request uses `"model-fast"`.

**`test_auto_route_selects_smart_for_complex_prompt`**
Same config. Send a 200-word prompt containing "refactor". Assert API request uses `"model-smart"`.

---

## 11. Docs Pages

### New: `docs/guide/model-switching.md`
Complete guide: session switching with `/model`, per-turn switching with `@model`, named shortcuts `/fast` and `/smart`, auto-routing configuration and tuning, cost tracking across switches, provider requirements, examples for common workflows (search → refactor → summarize pattern), `CLIDO_LOG=debug` routing decision inspection.

### Update: `docs/guide/tui.md`
Add section "Model switching in TUI" covering: header model display, slash commands, `@model` inline syntax, auto-route info lines in transcript.

### Update: `docs/reference/slash-commands.md`
Add `/fast` and `/smart` to the command table. Update `/model` row to document the `<name>` argument.

### Update: `docs/reference/configuration.md`
Add `[agent]` section rows for `auto_route`, `fast_model`, `smart_model`. Add `capability_tier` column to the models reference table.

---

## 12. Definition of Done

- [ ] `AgentLoop` has `model_stack: Vec<String>`, `per_model_cost: HashMap<String, f64>`, `auto_route: bool`, `fast_model: Option<String>`, `smart_model: Option<String>` fields.
- [ ] `push_model()` and `pop_model()` implemented and tested; `current_model()` always returns `model_stack.last()`.
- [ ] `/model <name>` TUI slash command switches the session model and emits `AgentEvent::ModelSwitch`.
- [ ] `/fast` TUI slash command resolves and switches to the cheapest fast-tier model.
- [ ] `/smart` TUI slash command resolves and switches to the most capable smart-tier model.
- [ ] `@model-name <prompt>` per-turn syntax parsed in TUI input handler; model reverts after turn.
- [ ] Auto-routing heuristic implemented in `routing.rs` with `score_complexity()` and `route()` functions.
- [ ] `auto_route = true` config activates per-turn routing; routing decision emitted as `AgentEvent::ModelSwitch` with `AutoRoute` reason.
- [ ] `capability_tier` field present in `pricing.toml` for all models; `/fast` and `/smart` resolve dynamically from this field.
- [ ] Provider validation runs on any model switch; friendly error emitted before API call if provider not configured.
- [ ] Conversation history fully preserved across all three switch modes.
- [ ] Per-model cost tracking implemented; `/cost` command shows per-model breakdown.
- [ ] TUI header updates immediately on `AgentEvent::ModelSwitch`.
- [ ] All 17 unit and integration tests pass in CI.
- [ ] `docs/guide/model-switching.md` written and linked from sidebar.
- [ ] `docs/reference/slash-commands.md` updated with `/fast`, `/smart`, and updated `/model`.
- [ ] `docs/reference/configuration.md` updated with `auto_route`, `fast_model`, `smart_model`.
