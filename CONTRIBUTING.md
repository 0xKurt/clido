# Contributing to cli;do

Thank you for your interest in contributing!

## Getting started

### 1. Fork and clone

```bash
git clone https://github.com/clido-ai/clido-cli.git
cd clido-cli
```

### 2. Set up Rust

This repo pins **Rust 1.94** in `rust-toolchain.toml`. Use **rustup** (install from https://rustup.rs):

```bash
rustup update
```

### 3. Build and test

```bash
cargo build --workspace
cargo test --workspace
```

### 4. Pre-commit hook (recommended)

```bash
git config core.hooksPath .githooks
```

The hook runs `cargo fmt --check` and `cargo clippy --workspace -- -D warnings` before each commit.

**If the hook fails:**
- `cargo fmt --check` failed → run `cargo fmt --all`, then `git add -u` and retry
- `cargo clippy` failed with E0514 → run `rustup update`, then `export PATH="${HOME}/.cargo/bin:${PATH}"` and `unset RUSTC`, then `cargo clean` and retry

## Branch naming

| Category | Pattern | Example |
|----------|---------|---------|
| Feature | `feature/<description>` | `feature/workflow-editor` |
| Bug fix | `fix/<description>` | `fix/session-resume` |
| Documentation | `docs/<description>` | `docs/add-mcp-guide` |

## Project structure

See [docs/developer/crates.md](/docs/developer/crates.md) for a full breakdown of the workspace crates.

```
crates/
  clido-cli/       — TUI, setup, command registry
  clido-agent/     — Agent loop, planning
  clido-tools/     — Built-in tools (Bash, Read, Write, etc.)
  clido-providers/ — LLM provider implementations
  clido-core/      — Shared types and config
  clido-storage/   — Session persistence
  clido-memory/    — FTS5 memory system
  clido-index/     — Repo indexing
  clido-workflows/ — YAML workflow engine
  clido-planner/   — Task planner
  clido-checkpoint/— Checkpoint/rollback
  clido-context/   — Context management
```

## Developer docs

- [Architecture](/docs/developer/architecture.md)
- [Adding providers](/docs/developer/adding-providers.md)
- [Adding tools](/docs/developer/adding-tools.md)
- [Crate guide](/docs/developer/crates.md)
