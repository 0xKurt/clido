# Introduction

**clido** is an AI coding agent for your terminal. Give it a task in plain English and it will read your codebase, write code, run tests, fix errors, and report back — all inside a single conversation.

It is built in Rust and ships as a single statically-linked binary with no runtime dependencies.

## What clido does

clido connects a large language model to a set of tools that can interact with your filesystem and shell:

| Tool | What it does |
|------|-------------|
| `Bash` | Runs shell commands with timeout and sandboxing support |
| `Read` | Reads file contents with optional line-range slicing |
| `Write` | Creates or overwrites files |
| `Edit` | Applies precise string replacements to existing files |
| `Glob` | Finds files matching a pattern |
| `Grep` | Searches file content with regex |
| `SemanticSearch` | Searches your indexed repository by symbol or concept |

The agent loops autonomously — it calls tools, observes results, and continues until it completes the task, hits a turn limit, or exceeds a cost budget.

## Key capabilities

- **Interactive TUI** — a Ratatui-based terminal UI with a chat pane, real-time tool progress, permission prompts, and a cost/token status strip
- **Persistent sessions** — every conversation is stored as a JSONL file; resume any session with `--continue` or `--resume <id>`
- **Long-term memory** — facts extracted from sessions are stored in SQLite and injected into future conversations automatically
- **Repository index** — build a file and symbol index to enable fast semantic search across large codebases
- **Workflows** — declare multi-step agent pipelines in YAML with dynamic parameters and parallel steps
- **MCP servers** — connect external tools over the Model Context Protocol
- **Multiple providers** — Anthropic, OpenAI-compatible endpoints, OpenRouter, and local Ollama models
- **Audit log** — every tool call is logged with inputs, outputs, and timing for review

## How it compares

| Feature | clido | Claude Code | Aider | Continue |
|---------|-------|-------------|-------|----------|
| Language | Rust | TypeScript | Python | TypeScript |
| TUI | Yes | Yes | Yes | Editor extension |
| Persistent sessions | Yes | Yes | Partial | No |
| Workflows | Yes | No | No | No |
| MCP support | Yes | Yes | No | Yes |
| Local models | Yes | No | Yes | Yes |
| Single binary | Yes | No | No | No |

clido is closest to Claude Code in its agentic scope, but is provider-agnostic and ships as a self-contained binary. It is designed to be embedded in CI pipelines and scripts as well as used interactively.

## A first look

Start a session in the current directory:

```bash
$ clido "refactor the parse() function in src/parser.rs to return Result<T, ParseError>"
```

clido reads the file, plans the change, applies it with the Edit tool, runs `cargo check`, and summarises what changed:

```
[Turn 1] Reading src/parser.rs...
[Turn 2] Applying edit to src/parser.rs...
[Turn 3] Running cargo check...
[Turn 4] All checks pass. Refactor complete.

  Modified: src/parser.rs
  Cost: $0.0021  Turns: 4  Time: 8.3s
```

Or open the interactive TUI:

```bash
$ clido
```

```
╭─ clido ──────────────────────────── claude-3-5-sonnet ─╮
│                                                         │
│  Hello! I'm ready to help with your Rust project.      │
│  What would you like to work on?                        │
│                                                         │
╰─────────────────────────────────────────────────────────╯
> _
```

## Next steps

- [Install clido](/guide/installation) — build from source and verify your setup
- [Quick Start](/guide/quick-start) — run your first prompt in five minutes
- [TUI Guide](/guide/tui) — learn the interactive interface
- [Configuration](/guide/configuration) — set your provider and model
