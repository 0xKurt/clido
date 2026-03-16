# Idea: Multi-Model Subagent Orchestration

## Status

Planned for **V3** (Stage 1: per-subagent model override) and **V4** (Stage 2: orchestrator decomposition guidance). Builds on Phase 4.7 (Subagent Architecture, V3) from the main roadmap.

## The Problem With Single-Model Agents

Clido's current design uses one model for the entire session. Every turn — exploration, planning, editing, reviewing, testing — is handled by the same model at the same cost and the same latency.

This is wasteful, slow, and often wrong. Different tasks have radically different requirements:

| Task | What matters | What a typical strong model gives you |
|------|-------------|---------------------------------------|
| File exploration | Speed, low cost | Expensive overkill |
| Code generation | Reasoning quality | Appropriate |
| Syntax fix | Speed, accuracy | Often overkill |
| Critical code review | Deep reasoning, caution | Appropriate |
| Summarization | Fast, cheap | Expensive overkill |
| Security analysis | Highest accuracy, caution | Appropriate or underpowered |
| Test generation | Pattern repetition, speed | Often overkill |
| Documentation | Writing quality | Varies |

Using `claude-opus-4-6` to list files or generate a docstring is like hiring a senior architect to paint a room. It works, but the cost and time are wrong for the job.

## The Core Idea

Route subagents to the model that is best suited for their task class.

The parent agent — the orchestrator — runs on a capable reasoning model. It breaks the task into subtasks and delegates each to a subagent configured with the appropriate model for that subtask. Each subagent completes its work autonomously and returns a result to the orchestrator.

This makes Clido simultaneously:
- faster, because lightweight tasks do not wait on expensive inference
- cheaper, because small models handle appropriate work
- more accurate, because specialized or stronger models handle critical work
- more parallelizable, because subagents run concurrently under independent models

## Model Classes

The idea is not to enumerate every possible model. It is to define a small number of task classes and map appropriate model tiers to them.

### Class: Fast

For routine mechanical tasks where correctness is simple to verify and reasoning depth is not needed.

Examples:
- list files matching a pattern
- rename a variable across a file
- generate boilerplate from a template
- format or lint a code block
- extract a section of a file
- summarize a file in one sentence

Model characteristics:
- low latency (target: < 1 second to first token)
- low cost per token
- sufficient instruction-following ability
- does not need deep reasoning

Candidate models: small local models via Ollama, `claude-haiku`, `gpt-4o-mini`, `qwen-turbo`.

### Class: Standard

For the majority of coding tasks requiring moderate reasoning.

Examples:
- implement a function from a description
- write unit tests for existing code
- generate a commit message or changelog entry
- explain what a module does
- apply a documented refactor pattern

Model characteristics:
- moderate latency and cost
- strong instruction following
- solid reasoning for typical code patterns
- no need for extended chain of thought

Candidate models: `claude-sonnet`, `gpt-4o`, `qwen-plus`.

### Class: Strong

For tasks that require deep reasoning, careful judgment, or high-stakes decisions.

Examples:
- architecture review
- security vulnerability analysis
- complex multi-step refactors
- evaluating trade-offs between implementations
- deciding whether to accept or reject a large change
- evaluating test coverage quality

Model characteristics:
- higher latency acceptable (quality trumps speed)
- higher cost acceptable
- extended context useful
- deep reasoning essential

Candidate models: `claude-opus`, `o3`, `qwen-max`.

### Class: Specialized

For tasks where a domain-specific or fine-tuned model outperforms general-purpose models.

Examples:
- formal proof or verification tasks
- mathematical reasoning
- domain-specific code (Solidity, hardware description languages, etc.)
- code completion in specific languages

This class is handled by routing to models specifically configured or fine-tuned for the domain.

## How The Orchestration Works

### The orchestrator model

The parent agent runs the primary reasoning loop. It:
- understands the task
- identifies natural decompositions
- assigns each subtask to the right model class
- waits for or merges results
- applies final judgment

The orchestrator model should generally be in the **strong** class. It is doing coordination and final decision-making.

### Subagent dispatch

When the orchestrator identifies a subtask, it issues an `AgentTool` call (Phase 4.7) with a `model_class` hint or an explicit model override:

```json
{
  "prompt": "Read src/main.rs and summarize what each function does in one line each.",
  "model_class": "fast",
  "max_turns": 3
}
```

Clido's SubAgentManager maps `model_class` to the configured model for that class and spins up the subagent on the appropriate provider.

### Parallel dispatch

When the orchestrator identifies multiple independent subtasks, it can issue several `AgentTool` calls in a single turn. Clido's parallel tool execution (Phase 4.6) then runs these concurrently, each on their own model.

Example: the orchestrator asks three `fast` subagents to read different files simultaneously, then feeds all three summaries into a single `strong` subagent for synthesis.

### Result integration

The orchestrator receives all subagent results as tool outputs in its history. It applies its own reasoning to integrate, evaluate, and decide what to do next.

This gives the strong model what it is actually good at: judgment and synthesis, not mechanical reading.

## The Developer Workflow Acceleration Pattern

This idea is particularly powerful for development review workflows:

```
User: "Review this pull request and tell me what concerns you most."

Orchestrator (strong model):
  → spawns fast subagents to read each changed file in parallel
  → spawns standard subagents to generate a per-file change summary
  → spawns strong subagent to perform security review of auth changes
  → spawns standard subagent to check test coverage
  → receives all results
  → synthesizes a final review with priority-ordered concerns
```

Compare this to a single-model session where the same model reads every file sequentially, generates summaries one at a time, and then tries to reason about everything at once with a bloated context window.

The multi-model approach is faster, cheaper on the mechanical parts, and more accurate on the critical parts.

## Architecture Changes Required

Phase 4.7 as currently designed shares the parent's `Arc<dyn ModelProvider>` with all subagents. That needs to extend to support per-subagent model selection.

### Provider selection in SubAgentConfig

```rust
pub struct SubAgentConfig {
    pub prompt: String,
    pub description: Option<String>,
    pub subagent_type: SubAgentType,
    pub max_turns: u32,
    // New: override the model for this subagent
    pub model_override: Option<String>,
    // New: use a named model class from config
    pub model_class: Option<ModelClass>,
}

pub enum ModelClass {
    Fast,
    Standard,
    Strong,
    Specialized(String),  // named specialization from config
}
```

### Model class configuration

Users configure model class mappings in `~/.config/clido/config.toml`:

```toml
[model_classes]
fast = "claude-haiku-3"
standard = "claude-sonnet-4-5"
strong = "claude-opus-4-6"

[model_classes.specialized]
solidity = "anthropic/claude-opus-4-6"  # or a fine-tuned model
math = "openai/o3"
```

### SubAgentManager model selection

```rust
impl SubAgentManager {
    fn resolve_provider(&self, config: &SubAgentConfig) -> Arc<dyn ModelProvider> {
        if let Some(model) = &config.model_override {
            self.provider_factory.build_for_model(model)
        } else if let Some(class) = &config.model_class {
            let model = self.config.model_classes.resolve(class);
            self.provider_factory.build_for_model(&model)
        } else {
            Arc::clone(&self.default_provider)
        }
    }
}
```

### AgentTool extension

The `AgentTool` tool schema adds optional fields:

```json
{
  "name": "Agent",
  "parameters": {
    "prompt": "string (required)",
    "description": "string (optional)",
    "subagent_type": "string (optional)",
    "model_class": "fast | standard | strong | specialized:{name} (optional)",
    "model": "string — explicit model name override (optional)"
  }
}
```

The model or class selection appears in the session audit trail for observability.

## Cost and Latency Model

The economics of this design depend on how well the orchestrator decomposes work.

Example task: "Audit this 50-file Rust repository for security issues."

**Without model routing (single strong model):**
- 50 sequential file reads: 50 × ~2s = ~100s
- 50 × strong model cost per read
- 1 synthesis: strong model cost
- Total: slow, expensive

**With model routing:**
- 50 parallel fast subagents for file reading: ~2s total (concurrent)
- 50 × fast model cost per read (10–20× cheaper per token)
- 1 strong subagent for security synthesis of the summaries
- Total: much faster, much cheaper on the exploratory work, same quality on the critical synthesis

The quality trade-off is: fast models reading files and summarizing them may miss subtle signals. The orchestrator must be designed to request relevant excerpts rather than only summaries when a strong-model step needs direct evidence.

## Failure Modes and Mitigations

### Fast model produces wrong output

The orchestrator, running a stronger model, can detect shallow or incorrect subagent output and retry with `model_class: standard` or `model_class: strong`. This retry logic can be built into the orchestrator's prompt guidance.

### Model class misconfiguration

A user who maps `fast` to a model that cannot follow tool schemas will get broken subagent behavior. The `clido doctor` command (Phase 8.4) should validate that all configured model classes support tool use.

### Cost explosion from many subagents

The same semaphore used for tool parallelism (Phase 4.6) bounds subagent concurrency. Each subagent has its own turn limit. The parent's `max_budget_usd` propagates across all spawned subagents.

### Orchestrator over-delegation

If the orchestrator spawns too many subagents for tasks that do not benefit from decomposition, overhead dominates. The system prompt for the orchestrator must include guidance on when decomposition is beneficial.

## Integration With Other Ideas

### Skills and workflows marketplace

A workflow definition (see `skills-workflows-marketplace-and-agent-payments.md`) can declare which model class is recommended for each step. When Clido executes a purchased workflow, it uses the declared model class per step.

### Self-improvement loops

Self-evaluation tasks are a natural fit for strong-model subagents. After a session, Clido can spawn a strong-model subagent to review its own trace and produce an improvement report. See `self-improvement-loops.md`.

### Repository indexing

When Clido's index (Phase 8.7) returns candidate files, it can route the "read and summarize" step for each candidate to a fast subagent, then route the synthesis step to a strong subagent.

## Implementation Priority

This idea should be built in two stages:

**Stage 1: Per-subagent model override (V3 addition)**

Extend `SubAgentConfig` to accept `model_override` and `model_class`. Build the provider resolution logic. Expose through `AgentTool` schema. This is a small additive change to the Phase 4.7 implementation.

**Stage 2: Orchestrator-level decomposition guidance (V3 or V4)**

Update the system prompt and tool guidance to teach the orchestrator when and how to use model classes. Build the cost-tracking integration so subagent costs are aggregated into the parent session's budget tracking.

## Open Questions

- Should the orchestrator model be configurable, or should Clido always use the `strong` class for the parent loop?
- Should model class mappings be per-project (in `CLIDO.md`) or only global (in `config.toml`)?
- How should the orchestrator's prompt guidance teach it to choose model classes without becoming brittle?
- Should Clido expose a `--model-class` CLI flag for the parent loop, or only through config?
- When a fast subagent fails, should the retry automatically escalate to standard or require orchestrator awareness?
- How do we expose per-subagent cost breakdown in session telemetry?

## Summary

Multi-model subagent orchestration makes Clido fundamentally more efficient on complex, multi-part tasks.

The orchestrator uses the strongest model for judgment and coordination. The mechanical work — reading files, generating boilerplate, reformatting, summarizing — goes to fast, cheap models in parallel.

The result is faster wall-clock time, lower cost on routine steps, and no quality regression on critical steps.

This is one of the clearest ways Clido can outperform single-model agents on real-world tasks at scale.
