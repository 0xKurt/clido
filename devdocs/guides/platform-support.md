# Platform support and packaging matrix

This document is the concise reference for **supported platforms**, **partial support**, and **not yet supported** by release, plus packaging and known degradation paths.

---

## Support levels

| Level | Meaning |
|-------|--------|
| **Supported** | Tested in CI; all core features (agent loop, six tools, sessions, doctor) work. |
| **Partial** | Core works with documented limitations (e.g. Bash degraded on Windows). |
| **Not yet** | Out of scope for that release or explicitly deferred. |

---

## By platform and release

| Platform | V1 | V1.5 | V2 |
|----------|----|------|-----|
| **macOS** (Intel, Apple Silicon) | Supported | Supported | Supported; `--sandbox` best-effort (sandbox-exec deprecated on 14+) |
| **Linux** (glibc, musl) | Supported | Supported | Supported; `--sandbox` (seccomp/Docker) primary hardened path |
| **Windows** | Partial (BashTool degraded) | Partial | Partial; no OS sandbox; PowerShell mode deferred |

### Windows details (V1)

- **BashTool:** No POSIX shell by default. Clido detects `bash` (Git Bash), WSL, or `sh` on PATH. If none found, `BashTool::execute()` returns an error with a clear message; no panic.
- **doctor:** V1 includes a check for POSIX shell availability; if absent, doctor reports a **warning** (not a failure).
- **CI:** Windows build runs full test suite with BashTool in degraded mode (mock/skip shell-dependent tests); Read, Write, Edit, Glob, Grep must pass.
- **Future:** V2+ may add a first-class PowerShell execution mode.

Reference: [development-plan.md](../plans/development-plan.md) Phase 9.5 (Windows strategy), Phase 7.1 (sandboxing).

---

## Packaging and distribution

| Method | V1 | V1.5 | V2 |
|--------|----|------|-----|
| **cargo install** (crates.io) | Not required for V1 exit criteria | Optional | Yes; verified |
| **Static binary (Linux musl)** | — | — | Yes (e.g. x86_64-unknown-linux-musl) |
| **macOS universal binary** | — | — | Yes (x86_64 + aarch64, lipo) |
| **Homebrew** (tap) | — | — | Yes; formula published |
| **Windows installer / binary** | — | — | As needed for Tier 2 |

Reference: [development-plan.md](../plans/development-plan.md) Phase 9.5 (Packaging and Distribution).

---

## Completions and docs

| Feature | V1 | V1.5 | V2 |
|---------|----|------|-----|
| Shell completions (bash, zsh, fish) | No | No | Yes (Phase 8.5) |
| Man pages | No | No | Yes |
| Structured telemetry | No | No | Yes |

---

## Sandbox behavior by OS (V2)

| OS | `--sandbox` | Notes |
|----|-------------|--------|
| macOS | Best-effort | `sandbox-exec`; deprecated on macOS 14+; document in user docs. |
| Linux | Primary | seccomp-bpf or optional `--docker-sandbox`. |
| Windows | Not implemented | No sandbox in V1/V2; future work. |

Reference: [security-model.md](security-model.md) §8; [development-plan.md](../plans/development-plan.md) Phase 7.1.
