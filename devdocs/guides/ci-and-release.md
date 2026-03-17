# CI and Release

This guide defines versioning policy, GitHub Actions CI lanes, nextest configuration, `.cargo/config.toml`, cargo deny policy, and fixture repository contents.

---

## 1. Versioning policy

- **V1:** Ships as version **0.1.0**. This is the first usable release but not yet "stable" for semantic versioning purposes.
- **1.0.0:** Reserved for the first publicly announced stable release (e.g. after V2 or when the maintainers declare API stability).
- **Thereafter:** Semantic versioning: MAJOR.MINOR.PATCH. Bump version in all workspace `Cargo.toml` files when cutting a release. Document each bump in `CHANGELOG.md`.

This policy affects crates.io publishing (Phase 9.5.3), Homebrew formula versioning, and upgrade messaging.

---

## 2. GitHub Actions CI lane structure

| Lane | Trigger | Steps | Secrets |
|------|---------|--------|---------|
| **Fast** | Every push | `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo nextest run --workspace` (default profile) | None |
| **Extended** | main branch (or on demand) | Full integration suite, property tests, resilience tests, performance smoke checks | None |
| **Live-provider** | Scheduled or protected branch | Real API compatibility tests; tagged `[live]` in test names | `ANTHROPIC_API_KEY` (optional; skip if unset) |
| **Release gate** | Release workflow | Matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`; build binaries; run packaging smoke tests; upload artifacts | As needed for publishing |

**Cache strategy:** Cache `~/.cargo/registry` and `target/` with keys that include `Cargo.lock` (and optionally rust toolchain) so dependency builds are reused.

**Secret names:** Use `ANTHROPIC_API_KEY` for live-provider tests. Do not commit secrets; document required secrets in this guide or in the workflow file comments.

---

## 3. nextest.toml configuration

Use [cargo-nextest](https://nexte.st/) for faster, parallel test runs. Recommended configuration:

- **Profile `default` (fast lane):** Run unit and integration tests that do not need network or special fixtures. No retries.
- **Profile `extended`:** Include slower integration tests, property tests, resilience tests. Optional: `retries = { backoff = "fixed", count = 1 }` for known-flaky resilience tests only.
- **Test thread count:** Leave at nextest default (based on CPUs); or set `concurrency = 4` in CI if the runner is shared.

Example (`.config/nextest.toml` in repo root or in CI config):

```toml
[profile.default]
# default: all tests, no retries

[profile.extended]
# same as default but used when running extended lane
# retries = { backoff = "fixed", count = 1 }
```

Live-provider tests are excluded from the default run (e.g. `cargo nextest run --workspace --exclude-feature live`) or gated by `#[cfg(feature = "integration")]`.

---

## 4. .cargo/config.toml content

Create `.cargo/config.toml` in the workspace root with the following so that builds are reproducible and dev iteration is fast:

```toml
[build]
# Use one target directory for the whole workspace (default)

[profile.release]
lto = "thin"
codegen-units = 1

[profile.dev]
opt-level = 1
# Faster incremental builds than default opt-level = 0
```

- **Release:** `lto = "thin"` gives good optimization without excessive link time; `codegen-units = 1` improves runtime performance.
- **Dev:** `opt-level = 1` speeds up debug builds while keeping compile times reasonable.

Additional target or toolchain settings (e.g. for `musl` or Windows) can be added when packaging (Phase 9.5).

---

## 5. cargo deny policy

Phase 9.6.3 requires `cargo deny check` with a policy file. Create `deny.toml` (or equivalent) in the repo root.

- **Licenses:** Allow the following: `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`. Deny all others by default, or list explicitly.
- **Banned crates:** None initially; add only if the project adopts a policy (e.g. no unmaintained crates).
- **Advisories:** Set to `deny` so that known CVEs in dependencies fail the check.

Example structure (exact syntax follows [cargo-deny](https://embarkstudios.github.io/cargo-deny/) docs):

```toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-DFS-2016"]
deny = []
```

---

## 6. Fixture repository contents

These fixtures are used by the integration test suite (Phase 5.4.1) and must have defined contents.

### tests/fixtures/sample-project/

**Purpose:** Minimal valid Rust project for happy-path agent runs and basic tool tests.

**Contents:**

- `Cargo.toml` — binary + lib, one dependency if needed (e.g. nothing or `anyhow`).
- `src/main.rs` — calls a function from the lib and prints.
- `src/lib.rs` — two functions: `greet(name: &str) -> String` and `add(a: i32, b: i32) -> i32` (or similar). One has a corresponding test that passes, one that fails (e.g. wrong expected value) so the agent can "fix the failing test" in tests.

**Intentional state:** Compiles; one failing test. Used for: Read/Glob/Grep, Edit to fix test, Bash to run `cargo test`.

### tests/fixtures/broken-project/

**Purpose:** Intentional compile error for error-recovery and doctor/validation tests.

**Contents:** Same structure as `sample-project/` except `src/lib.rs` contains one deliberate syntax error (e.g. missing `}` or invalid token). Used for: agent error recovery, validation that Clido does not silently ignore build failures when asked to fix code.

### tests/fixtures/session-fixtures/

**Purpose:** Pre-recorded session JSONL for resume and session-format tests.

**Contents:**

- `single-turn.jsonl` — one user message, one assistant text response, no tools.
- `multi-turn-with-tool.jsonl` — user message, assistant tool call (e.g. Read), tool result, assistant text response.
- `interrupted.jsonl` — same as multi-turn but truncated mid-turn (no final Result line) to test resume and stale-file handling.

Schema must match `devdocs/schemas/session.md`. Each file is a valid sequence of SessionLine JSON objects, one per line.

### tests/fixtures/large-project/

**Purpose:** Performance and search behavior on a larger repo.

**Not committed.** Generated at CI time (or locally when needed): e.g. clone or symlink a medium-sized open-source Rust repo (e.g. a known commit of `ripgrep` or `fd-find`) into `tests/fixtures/large-project/`. CI workflow checks out the repo as a step. This avoids storing a large tree in the clido repo.

### tests/fixtures/planner-fixtures/ (V4)

**Purpose:** Task-graph and planner tests when the planner is implemented.

**Contents:** To be defined in V4; e.g. pre-recorded TaskGraph JSON and expected execution order. Can be added when Phase 4.8 is implemented.
