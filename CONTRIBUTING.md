# Contributing to cli;do

Thank you for your interest in contributing. This document explains where to start, which docs are authoritative, and what is intentionally out of scope for V1.

## Which document wins on conflicts

- **User-facing behavior (CLI, flags, exit codes, output format):** The [CLI interface specification](devdocs/plans/cli-interface-specification.md) is the authority. If the development plan or any other doc disagrees with it, the CLI spec wins and the other doc should be updated.
- **Implementation sequence and milestones:** The [development plan](devdocs/plans/development-plan.md) is the authority. Implement in the order it specifies; each phase has clear dependencies.
- **If you find a contradiction:** Open an issue, fix the doc first (so the spec is consistent), then implement. Do not implement to one doc while another says something different.

## Where implementation starts

Start at **Phase 1** of the development plan: workspace init, core types, tracing. The first commands to run once the workspace exists:

```bash
cargo build --workspace
cargo test --workspace
cargo nextest run --workspace
```

Use the [local development testing](devdocs/guides/local-development-testing.md) guide to run and test the agent without risking your own repositories.

## What is intentionally not implemented in V1

V1 ships a single working provider (Anthropic), the six core tools, sessions, config with named profiles, and basic doctor checks. The following are **not** in V1 and should not be assumed available:

- Multi-provider support (OpenAI, OpenRouter, Alibaba) — V2
- Subagents — V3
- Memory system — V3
- MCP support — V3
- Task graph / planner — V4
- Bash sandboxing — V2
- Telemetry and audit logging — V2
- Shell completion and man pages — V2
- Packaging and distribution — V2

See the [release plans](devdocs/plans/releases/README.md) for the full map of phases to releases.

## Resolving contradictions during implementation

If you discover that two docs conflict while implementing:

1. Prefer the CLI spec for any user-visible behavior.
2. Prefer the development plan for implementation order and internal design.
3. Update the losing doc in the same PR (or a follow-up) so the next contributor sees a single source of truth.
