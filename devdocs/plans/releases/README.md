# Clido Release Plans

Each release builds on the previous. Later releases are not planned in detail until earlier ones are shipped and measured.

## The Release Sequence

| Release | One-line Purpose |
|---------|-----------------|
| V1 | A real, usable agent on a single machine with one provider |
| V1.5 | Safe to automate, cheaper to run, easier to operate |
| V2 | Production-grade: multi-provider, packaged, benchmarked, documented |
| V3 | Advanced capabilities: subagents, memory, MCP, indexing, declarative workflows |
| V4 | Experimental: task graph planner for specific hard workflows |

## Files

- [`v1.md`](v1.md) — Core agent loop, six tools, sessions, context, permissions.
- [`v1-5.md`](v1-5.md) — Operator quality: cost tracking, parallelism, secret safety, machine-readable output.
- [`v2.md`](v2.md) — Product readiness: multi-provider, sandboxing, telemetry, packaging.
- [`v3.md`](v3.md) — Advanced platform: subagents, memory, MCP, repository indexing, declarative workflows.
- [`v4.md`](v4.md) — Planner and experimental orchestration for complex task types.

## Roadmap Coverage

Every phase from `development-plan.md` is assigned to exactly one release:

| Roadmap Phase | Release |
|---------------|---------|
| Phase 1 — Foundation | V1 |
| Phase 2 — Proof of Concept | V1 |
| Phase 3 — Minimal Viable Agent | V1 |
| Phase 4.2 — Context Engine | V1 |
| Phase 4.3 — Permission System | V1 |
| Phase 4.5 — Plan Mode | V1 |
| Phase 5.1 — Robust Error Handling | V1 |
| Phase 5.2 — Session Recovery | V1 |
| Phase 5.3 — Graceful Shutdown | V1 |
| Phase 5.4 — Integration Test Suite | V1 |
| Phase 5.6 — Edit Safety and Partial Write Detection | V1 |
| Phase 8.4 (basic) — `clido doctor` | V1 |
| Phase 4.4 — Cost Tracking | V1.5 |
| Phase 4.6 — Parallel Tool Execution | V1.5 |
| Phase 6.2 — Context Efficiency | V1.5 |
| Phase 7.2 — Secret Detection | V1.5 |
| Phase 8.2 — JSON and Stream-JSON Output | V1.5 |
| Phase 8.4 (expanded) — `clido doctor` (MCP, connectivity) | V1.5 |
| Phase 4.1 — Multi-Provider Support | V2 |
| Phase 4.2.4 — Prompt Caching | V2 |
| Phase 6.1 — Startup Performance | V2 |
| Phase 6.3 — Concurrent Provider Requests | V2 |
| Phase 6.4 — File Read LRU Cache | V2 |
| Phase 7.1 — Bash Sandboxing | V2 |
| Phase 7.3 — Audit Logging | V2 |
| Phase 8.1 — Hooks System | V2 |
| Phase 8.5 — Shell Completion and Man Pages | V2 |
| Phase 8.6 — Live Plan / Progress Visualization | V2 |
| Phase 9.1 — Full Test Coverage | V2 |
| Phase 9.2 — Benchmarks | V2 |
| Phase 9.3 — Structured Telemetry | V2 |
| Phase 9.4 — Documentation | V2 |
| Phase 9.5 — Packaging and Distribution | V2 |
| Phase 9.6 — Production Hardening | V2 |
| Phase 4.7 — Subagent Architecture | V3 |
| Phase 5.5 — Memory System | V3 |
| Phase 8.3 — MCP Support | V3 |
| Phase 8.7 — Repository Indexing | V3 |
| Phase 4.9 — Workflow Engine | V3 |
| Phase 4.9.x — Pre-Flight and Dynamic Parameters | V3 |
| Phase 4.8 — Task Graph / Planner | V4 |

### Competitive features (shipped ahead of roadmap schedule)

The following were implemented in the `feature/v1` branch to close the gap with Cursor / Claude Code / Cline. They map loosely to later roadmap phases but were pulled forward due to high standalone value.

| Feature | Crate(s) touched | Nominal roadmap home |
|---------|-----------------|---------------------|
| Project Rules (`CLIDO.md` hierarchy) | `clido-cli`, `clido-context` | Phase 3.4.2 |
| `.gitignore`-aware Repository Index | `clido-index` | Phase 8.7 (V3) |
| Edit Tool Multi-Strategy Patching | `clido-tools` | Phase 5.6 (V1) |
| Diff Preview and Approval Before Write | `clido-tools`, `clido-cli` | — |
| Interactive Plan Mode with TUI Editor | `clido-planner`, `clido-cli` | Phase 4.5 / 4.8 |
| Checkpoint and Rollback | `clido-checkpoint` | — |
| Web Fetch and Search Tools | `clido-tools` | — |
| Native Git Awareness | `clido-tools`, `clido-context` | — |
| Desktop Notifications and Completion Hooks | `clido-cli` | — |
| Automatic Test Loop | `clido-agent`, `clido-cli` | — |
| LSP / Compiler Diagnostics Tool | `clido-tools` | — |
| Mid-Session Model Switching | `clido-cli` | — |
| Image and Screenshot Input | `clido-tools`, `clido-cli` | — |

## Planning Principles

- The reactive agent loop is always the default execution model.
- Delay complexity that does not clearly improve task success.
- Prefer measurable quality improvements over architectural ambition.
- Optional systems are added only after the core loop is proven.
- Every V3 and V4 feature must define its success metric before implementation begins.

## Source of Truth

These release plans are derived from `devdocs/plans/development-plan.md`.
If there is a conflict between the two, update both documents together.

See also: `devdocs/guides/testing-strategy-and-master-test-plan.md` for testing priorities per release.
