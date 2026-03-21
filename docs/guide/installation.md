# Installation

clido is distributed as a Rust crate. There are no pre-built binaries yet, so you build from source using Cargo.

## Prerequisites

### Rust toolchain

clido requires a recent stable Rust toolchain. The minimum supported version is specified in `rust-toolchain.toml` in the repository root.

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify your toolchain:

```bash
rustc --version   # should print rustc 1.78.0 or later
cargo --version
```

### API key

You need an API key for at least one supported provider:

| Provider | Key variable | Where to get one |
|----------|-------------|------------------|
| Anthropic (recommended) | `ANTHROPIC_API_KEY` | [console.anthropic.com](https://console.anthropic.com) |
| OpenAI-compatible | `OPENAI_API_KEY` | Your provider's dashboard |
| OpenRouter | `OPENROUTER_API_KEY` | [openrouter.ai](https://openrouter.ai) |
| Local (Ollama) | — | [ollama.ai](https://ollama.ai) — no key needed |

You can store the key in your shell profile or enter it during `clido init`.

## Build from source

Clone the repository and install with Cargo:

```bash
git clone https://github.com/kurtbuilds/clido.git
cd clido
cargo install --path crates/clido-cli
```

This compiles the workspace and copies the `clido` binary into `~/.cargo/bin/`, which should already be on your `PATH` if you used rustup.

::: tip Release build
The default `cargo install` builds in release mode. If you want a debug build for development, use `cargo build --workspace` and run `./target/debug/clido` directly.
:::

### Build with all features

```bash
cargo install --path crates/clido-cli --locked
```

The `--locked` flag ensures the exact dependency versions from `Cargo.lock` are used, which is recommended for reproducible builds.

## Verify the installation

```bash
clido --version
```

Expected output:

```
clido 0.1.0
```

Check that all required tools and configuration are present:

```bash
clido doctor
```

```
✓ Binary: clido 0.1.0
✓ API key: ANTHROPIC_API_KEY is set
✓ Config: ~/.config/clido/config.toml
✓ Session dir: ~/.local/share/clido/sessions
✓ Bash: /bin/bash
✓ All checks passed.
```

If any check fails, `doctor` will explain what to do. See the [First Run](/guide/first-run) guide for full setup details.

## Platform notes

### macOS

Fully supported on macOS 12+. The `--sandbox` flag uses `sandbox-exec` when available.

### Linux

Fully supported. The `--sandbox` flag uses `bwrap` (bubblewrap) when available. Install bubblewrap for sandboxed Bash execution:

```bash
# Debian / Ubuntu
sudo apt install bubblewrap

# Fedora / RHEL
sudo dnf install bubblewrap
```

### Windows

Windows is not currently supported. WSL2 with Ubuntu is a viable workaround.

## Shell completions

Generate completion scripts for your shell:

```bash
# Bash
clido completions bash >> ~/.bash_completion

# Zsh (add to ~/.zshrc)
clido completions zsh > "${fpath[1]}/_clido"

# Fish
clido completions fish > ~/.config/fish/completions/clido.fish
```

## Man page

Generate and install the man page:

```bash
clido man > /usr/local/share/man/man1/clido.1
man clido
```

## Updating

Pull the latest changes and reinstall:

```bash
cd clido
git pull
cargo install --path crates/clido-cli --force
```

## Next steps

- [Quick Start](/guide/quick-start) — run your first prompt
- [First Run](/guide/first-run) — configure a provider and API key
- [Configuration](/guide/configuration) — full config reference
