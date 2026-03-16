# cli;do

<p align="center">
  <img src="https://merbeth.io/files/clido.svg" width="420" height="140" alt="cli;do logo">
</p>

**cli;do** is a local-first, multi-provider CLI coding agent. Run it in your terminal, give it a task in plain language, and it uses AI (with tools like read, edit, search, and run) to get the job done—with permission prompts for anything that changes your files.

## Vision

- **CLI-first** — Built for the terminal; scripting and automation are first-class.
- **Multi-provider** — Use different AI backends (e.g. Anthropic, OpenAI) via profiles.
- **Safe by default** — Destructive or state-changing actions require your approval.
- **Session-aware** — Resume after interrupt; cost and usage visible when you care.

Planned capabilities include: core agent loop with tools, sessions, context and permissions (V1); JSON output and operator tooling (V1.5); multi-provider, sandboxing, packaging (V2); memory, MCP, semantic search (V3); optional task-graph planner (V4).

## Status

Early stage: architecture and product are specified in `devdocs/`. Implementation will follow a Rust workspace layout and the CLI interface specification.

## Documentation

| Doc | Description |
|-----|-------------|
| [Development plan](devdocs/plans/development-plan.md) | Architecture, Rust workspace, phased roadmap |
| [CLI interface spec](devdocs/plans/cli-interface-specification.md) | Canonical command surface and behavior |
| [Releases](devdocs/plans/releases/README.md) | V1 → V4 scope and exit criteria |

## License

See repository license (if any).
