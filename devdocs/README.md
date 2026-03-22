# Clido Developer Documentation

Design, architecture, schemas, and contributor guides for Clido.

## Start Here

- **[guides/implementation-bootstrap.md](./guides/implementation-bootstrap.md)** — Best entry point before writing code. Explains doc precedence, locked decisions, and build order.
- **[plans/cli-interface-specification.md](./plans/cli-interface-specification.md)** — Canonical user-facing CLI behavior. Source of truth for all flags, commands, and exit codes.
- **[plans/development-plan.md](./plans/development-plan.md)** — Architecture, crate structure, and milestone roadmap.

## Contributor Guides

- **[guides/local-development-testing.md](./guides/local-development-testing.md)** — Safe local development workflow with fixtures, local models, and development flags.
- **[guides/testing-strategy-and-master-test-plan.md](./guides/testing-strategy-and-master-test-plan.md)** — Testing strategy across unit, integration, e2e, performance, resilience, and security.
- **[guides/contributor-test-matrix.md](./guides/contributor-test-matrix.md)** — Test commands, required tools, and fast/slow lanes.
- **[guides/security-model.md](./guides/security-model.md)** — Security boundaries, permissions, path handling, secret redaction, and sandbox rules.
- **[guides/platform-support.md](./guides/platform-support.md)** — Platform support matrix and packaging expectations by release.
- **[guides/ci-and-release.md](./guides/ci-and-release.md)** — CI lanes, release validation, and packaging flow.
- **[guides/pricing-and-offline.md](./guides/pricing-and-offline.md)** — Pricing metadata, offline mode, and update behavior.
- **[guides/software-development-best-practices.md](./guides/software-development-best-practices.md)** — Project-wide engineering rules and documentation expectations.

## Schemas and References

- **[schemas/config.md](./schemas/config.md)** — `config.toml`, `.clido/config.toml`, and `pricing.toml`.
- **[schemas/session.md](./schemas/session.md)** — Session JSONL schema.
- **[schemas/output-and-session.md](./schemas/output-and-session.md)** — Output contracts, audit schemas, and versioning notes.
- **[schemas/types.md](./schemas/types.md)** — Shared type-level reference.
- **[algorithms/context-compaction.md](./algorithms/context-compaction.md)** — Context compaction algorithm: trigger threshold, summarization prompt, fallback, and hard-limit behavior.

## Roadmap

- **[plans/releases/README.md](./plans/releases/README.md)** — Release sequence V1 → V4 with phase-to-release mapping.
- **[plans/releases/v1.md](./plans/releases/v1.md)** — V1 scope and exit criteria.
- **[plans/releases/v1-5.md](./plans/releases/v1-5.md)** — V1.5 operator-quality scope.
- **[plans/releases/v2.md](./plans/releases/v2.md)** — V2 productization scope.
- **[plans/releases/v3.md](./plans/releases/v3.md)** — V3 advanced capability scope.
- **[plans/releases/v4.md](./plans/releases/v4.md)** — V4 planner scope.
- **[plans/ux-requirements.md](./plans/ux-requirements.md)** — UX and copy standards for interactive prompts, first-run, permission modals, and visual design.

## Research and Background

- **[REPORT.md](./REPORT.md)** — Reverse-engineering findings from Claude CLI and Cursor agent that informed Clido's design.

## Ideas

Exploratory — not binding unless promoted into the roadmap:

- **[ideas/multi-model-subagent-orchestration.md](./ideas/multi-model-subagent-orchestration.md)**
- **[ideas/self-improvement-loops.md](./ideas/self-improvement-loops.md)**
- **[ideas/skills-workflows-marketplace-and-agent-payments.md](./ideas/skills-workflows-marketplace-and-agent-payments.md)**
