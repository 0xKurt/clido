# Idea: Self-Improvement Loops

## Status

Planned for **V4**. Prerequisites: Phase 5.5 (Memory System, V3), Phase 4.7 (Subagent Architecture, V3), Phase 3.3 (Session Storage), Phase 4.2 (Context Engine), and project instructions loading (CLIDO.md).

## The Problem

Clido currently has no mechanism to learn from its own past performance.

Every session starts from the same baseline:
- the same system prompt
- the same tool guidance
- the same default behavior

If Clido makes a mistake — uses too many turns on a simple task, generates a wrong Edit, misses a critical security pattern — that information is recorded in the session file but nothing reads it. The next session starts identically.

This is a significant limitation. It means Clido cannot compound quality improvements over time. The humans using it improve; Clido does not.

Self-improvement loops close that gap. They give Clido a structured mechanism to review its own behavior, identify failure patterns, and update its operating context to perform better in the future.

## What Self-Improvement Means For An Agent

For a human developer, self-improvement means:
- reflecting on what went well and what did not
- noticing patterns in their mistakes
- updating their mental models and habits
- asking for help when something is unclear

For Clido, the equivalent is:
- analyzing session traces to find failure and inefficiency patterns
- identifying prompt or behavior patterns that consistently produce poor outcomes
- updating project instructions, memory, or skill configurations to correct those patterns
- flagging persistent uncertainties to the user for clarification

The critical difference: humans improve through experience over time. Clido's "experience" is its session history. The self-improvement loop reads that history, extracts signal, and produces updates that affect future behavior.

## Four Types of Self-Improvement

### 1. Session retrospective

After each session completes, Clido runs a lightweight analysis on its own turn history.

What it looks for:
- turns where the same tool was called more than twice in a row without progress
- Edit failures followed by retries
- tool calls that returned errors the agent should have anticipated
- turns where the session length significantly exceeded a reasonable estimate
- places where the model expressed uncertainty that was never resolved
- any `is_error: true` result that the model did not handle cleanly

Output: a brief retrospective note, stored in long-term memory with session metadata.

This retrospective is not shown to the user unless requested. It is raw material for the next level.

### 2. Pattern analysis across sessions

Periodically — or on request — Clido runs a deeper analysis across multiple session retrospectives.

What it looks for:
- recurring failure patterns (e.g., "Edit fails frequently on minified files")
- recurring inefficiencies (e.g., "Reads the same config file in every session before doing anything else")
- task categories where Clido consistently underperforms
- task categories where Clido consistently excels and could be pushed harder

Output: a structured pattern report, stored in long-term memory and optionally shown to the user.

### 3. Instruction update proposals

Based on pattern analysis, Clido proposes concrete changes to its own operating context.

Types of proposals:
- additions or changes to project instructions (`CLIDO.md`)
- additions or changes to tool guidance in the system prompt
- new memory entries that pre-load context Clido repeatedly has to rediscover
- new skill or workflow configurations

These proposals are always shown to the user before being applied. Clido cannot modify its own instructions without explicit approval.

Example proposal:
```
Pattern detected: In 7 of the last 10 sessions in this repository, Clido
read src/config.rs in turn 1 before doing anything else. This file contains
the project's configuration constants.

Proposed addition to CLIDO.md:
  "Always start sessions by reading src/config.rs to understand project
  configuration before exploring other files."

Apply this change? [y/N]
```

### 4. Capability gap identification

Clido identifies tasks it consistently cannot complete well and surfaces them to the user.

Output: a capability report listing task categories that repeatedly produce poor outcomes, with suggested solutions (new tools, different skill packs, model upgrades, human clarification).

Example:
```
Capability gap detected: Clido has failed to complete Solidity audit tasks
in 3 of 3 attempts in this repository. The failure pattern suggests Clido
lacks domain-specific knowledge about common Solidity vulnerabilities.

Suggested actions:
  1. Install a Solidity audit skill pack from the registry.
  2. Provide Clido with audit reference documentation in CLIDO.md.
  3. Use a specialized model for future Solidity tasks.
```

## The Self-Improvement Loop in Practice

A full cycle looks like this:

```
Session completes
  ↓
Retrospective analysis runs (lightweight, fast model)
  ↓
Retrospective stored in long-term memory
  ↓
[After N sessions or on request]
Pattern analysis runs across retrospectives (strong model)
  ↓
Pattern report generated
  ↓
Instruction update proposals generated
  ↓
User reviews and approves/rejects proposals
  ↓
Approved proposals applied to CLIDO.md or memory
  ↓
Future sessions start with updated context
```

The key property: Clido cannot modify its own behavior without human review. The loop produces proposals, not unilateral changes.

This is the right default. An agent that modifies its own instructions without oversight can drift in dangerous or unhelpful directions. Human approval keeps the loop aligned.

## What Counts as a "Bad" Session

The retrospective analysis needs a definition of poor performance. This should be configurable, but sensible defaults exist:

| Signal | Default threshold | Meaning |
|--------|------------------|---------|
| Turn count | > 2× estimated turns | Inefficient execution |
| Edit failure rate | > 25% of Edit calls fail | Poor file-reading before editing |
| Repeated identical tool calls | Same call 3+ times | Stuck in a loop |
| Session cost | > 2× expected cost for task class | Expensive relative to value |
| Explicit error in final result | Any | Task was not completed |
| User-provided negative feedback | Any | User indicated poor quality |

Users can adjust these thresholds in `config.toml`:

```toml
[self_improvement]
enabled = true
retrospective_after_session = true
pattern_analysis_after_n_sessions = 10
turn_count_multiplier_threshold = 2.0
edit_failure_rate_threshold = 0.25
```

## What Counts as a "Good" Session

The analysis should also recognize high-quality sessions to reinforce effective patterns:

- completed in fewer turns than average for similar tasks
- no tool errors
- user-provided positive feedback
- task verified as correct by tests or review

High-quality session patterns are also worth extracting. If Clido consistently performs well on a specific task structure, that structure can be codified into a workflow or skill.

## Concretely: What Does a Retrospective Look Like

Given a session JSONL file, the retrospective analysis produces a structured record.

Example:

```json
{
  "session_id": "abc123",
  "task_summary": "Refactor the authentication module to use the new token format.",
  "outcome": "completed",
  "quality_signals": {
    "total_turns": 14,
    "estimated_turns": 8,
    "edit_failures": 3,
    "edit_total": 9,
    "repeated_reads": 2,
    "cost_usd": 0.18,
    "estimated_cost_usd": 0.08
  },
  "identified_issues": [
    {
      "type": "edit_failure_pattern",
      "description": "3 Edit calls failed because the old_string did not match after a previous Edit changed the file. Clido should re-read the file after each Edit before issuing the next one.",
      "severity": "medium"
    },
    {
      "type": "repeated_read",
      "description": "src/auth/token.rs was read 3 times. Clido should cache or summarize file content in working memory rather than re-reading.",
      "severity": "low"
    }
  ],
  "proposed_improvements": [
    "Add to CLIDO.md: 'After each Edit, re-read the file before issuing the next Edit on the same file.'"
  ]
}
```

## How This Interacts With Memory

The memory system (Phase 5.5, V3) provides the storage layer.

- Retrospectives are stored as long-term memory entries tagged with session_id, task category, and quality signals.
- Pattern analysis queries the memory store for retrospectives across sessions.
- Approved instruction updates go into `CLIDO.md` (on-disk) or the persistent memory store for auto-injection into future session context.

Without the memory system, this feature has no persistence. Self-improvement loops require V3 as a prerequisite.

## The Meta-Session: Running Self-Analysis as an Agent Task

The retrospective and pattern analysis do not have to be deterministic code. They can themselves be agent tasks.

Clido spawns a read-only subagent — a "reflection agent" — with:
- access to session JSONL files (as Read tool inputs)
- a prompt that instructs it to analyze the session and output a structured retrospective
- no write access (it cannot modify the codebase it is reviewing)
- a strong or specialized model (quality of analysis matters here)

This is powerful because the analysis can be as nuanced as a human review. It uses language reasoning to interpret failure patterns, not just numeric thresholds.

The reflection agent's output feeds directly into the memory store.

For pattern analysis across sessions, a second reflection agent reads a batch of retrospectives from memory and produces the pattern report. This is an excellent use of the strong model class from `multi-model-subagent-orchestration.md`.

## The Instruction Proposal Agent

When Clido decides to propose an instruction update, it can use a third agent pass:

1. **Reflection agent** identifies a recurring problem.
2. **Proposal agent** drafts the minimal, specific change to `CLIDO.md` that addresses the problem.
3. **Human review** — the user approves, edits, or rejects the proposal.
4. **Application** — approved proposals are written to `CLIDO.md` using the `Edit` tool, producing a diff in the session record.

This creates a full audit trail: every change to Clido's operating instructions was proposed, reviewed, and approved by a human.

## An Honest Assessment of the Risks

### Risk: Prompt drift

If Clido proposes changes that are individually sensible but collectively incoherent, `CLIDO.md` can become a pile of contradictory instructions over time.

Mitigation: the proposal agent should always read the full current `CLIDO.md` before proposing a new addition. The user should periodically review and prune. Clido can flag when the instruction file has grown beyond a size threshold.

### Risk: Reinforcing the wrong behavior

If Clido consistently performs poorly on a task class but the retrospective analysis incorrectly diagnoses the cause, the proposed improvement may make things worse.

Mitigation: human review before any instruction change is applied. Optional A/B-style validation: run the old behavior and the proposed new behavior on the same fixture task before approving.

### Risk: False confidence

A session can look good on metrics (few turns, low cost, no errors) but produce a wrong result. The retrospective cannot verify semantic correctness without additional validation input.

Mitigation: use test results, compiler output, and user feedback as quality signals in addition to behavioral metrics. If a task produces a test failure after Clido says it is done, that counts as a quality signal.

### Risk: Self-serving optimization

Clido could optimize for metrics it controls (turn count, cost) at the expense of quality. Example: fewer turns by skipping necessary checks.

Mitigation: track quality independently of efficiency metrics. Include correctness signals (test results, user feedback, review outcomes) in the quality model, not just operational signals.

## What This Enables Over Time

If the self-improvement loop works well, Clido should become measurably better on a given project over multiple sessions.

Observable effects:
- turn count on similar tasks decreases over time as Clido pre-loads relevant context
- Edit failure rate decreases as instruction updates teach better read-before-edit habits
- task completion rate on domain-specific work improves as gaps are closed with skills or specialized models
- the user spends less time correcting Clido on project-specific conventions because those conventions are now in the instructions

This is what makes Clido different from a static agent. It is not just executing tasks. It is developing a working model of the project and its own performance over time.

## Integration With Multi-Model Orchestration

The self-improvement loop is a natural consumer of the multi-model subagent idea.

- **Fast model**: read session JSONL files and extract structured metrics
- **Standard model**: generate the retrospective analysis
- **Strong model**: identify cross-session patterns and draft instruction proposals

This keeps the cost of self-analysis proportional to its complexity.

## What Is Not Self-Improvement

To be precise about scope:

- **Not self-modification without approval.** Clido does not change its own code, tools, or prompts unilaterally.
- **Not online learning.** Clido does not fine-tune or modify model weights. Improvement is purely through context and instructions.
- **Not automatic.** Self-improvement loops run when triggered — after sessions, on request, or on schedule — not continuously in the background.
- **Not infallible.** The improvement proposals are AI-generated and require human judgment to evaluate.

## Implementation Prerequisites

Before self-improvement loops can work:
- long-term memory store (Phase 5.5, V3)
- session storage that records full traces (Phase 3.3, V1)
- project instruction loading from `CLIDO.md` (Phase 3.4.2, V1)
- subagent architecture for the reflection agent (Phase 4.7, V3)
- read-only subagent mode (already in Phase 4.7: `SubAgentType::ReadOnly`)

Minimum viable version: post-session retrospective stored in memory, pattern report on request, instruction proposals with human approval. This could ship as a V3 or V4 feature after memory and subagents are proven.

## Proposed Feature Scope

### Minimum viable

- `clido reflect` command: runs retrospective on the last session and stores the result
- `clido reflect --sessions N`: runs pattern analysis across the last N sessions
- `clido improve`: shows pending instruction proposals and allows user to approve or reject

### Full version

- automatic post-session retrospective (opt-in via config)
- scheduled or trigger-based pattern analysis
- capability gap reports
- diff-and-review workflow for instruction proposals
- integration with skills marketplace to suggest skill installs for detected gaps

## Open Questions

- Should retrospective analysis be synchronous (runs before Clido exits) or asynchronous (runs after Clido exits as a background process)?
- Should Clido propose changes to its system prompt directly, or only to `CLIDO.md`?
- How do we prevent instruction files from growing indefinitely? Should Clido be able to propose deletions as well as additions?
- How should user feedback be collected? Explicit rating? Implicit signals from follow-up behavior?
- Should the reflection agent run with the same model as the session, or always with the strong class?
- How do we handle retrospective analysis for sessions that used multiple models (from multi-model orchestration)?

## Summary

Self-improvement loops give Clido the ability to compound quality improvements over time by analyzing its own session history.

The loop is:
1. observe past behavior in session traces
2. identify patterns and failure modes
3. propose specific instruction changes
4. present proposals for human review
5. apply approved changes
6. measure improvement in future sessions

Clido does not modify itself. It proposes, humans decide, and outcomes are measured. That constraint is what makes the loop safe and trustworthy.

Over time, an Clido that runs self-improvement loops should become meaningfully better at the specific projects and task types it works on — not because the model changed, but because the context driving it is continuously refined by real experience.
