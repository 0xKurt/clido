# Contributor test matrix

This is the **operational** reference for running tests as a contributor: what the fast lane is, which commands run which test categories, what optional tools you need, and which feature flags and env vars gate expensive or live tests.

The strategic test plan is in [testing-strategy-and-master-test-plan.md](testing-strategy-and-master-test-plan.md). CI layout is in [ci-and-release.md](ci-and-release.md).

---

## 1. What “fast lane” is

The **fast lane** is the default test run that:

- Completes in a few minutes (no network, no live API).
- Runs on every push in CI.
- Includes: **unit tests** and **integration tests** that use mocks, in-memory providers, and local fixtures only. No real provider API calls, no expensive filesystem or concurrency stress tests.

**Command (local):**

```bash
cargo nextest run --workspace
```

If nextest is not installed:

```bash
cargo install cargo-nextest
```

**Command (without nextest):**

```bash
cargo test --workspace
```

Live-provider and other expensive tests are **excluded** from the fast lane (see §4).

---

## 2. Optional tools (local)

| Tool | Purpose | Required for fast lane? |
|------|---------|-------------------------|
| **cargo-nextest** | Parallel test runner; used in CI and recommended locally | No; `cargo test` works, but nextest is faster |
| **tarpaulin** (or **cargo-llvm-cov**) | Code coverage | No; used in extended/coverage jobs |
| **valgrind** (Linux) / **Instruments** (macOS) | Memory/performance profiling | No; used for manual or scheduled profiling |

None of these are required to run the fast lane. Installing `cargo-nextest` is recommended for consistency with CI.

---

## 3. Which commands run which test categories

| Intent | Command | What runs |
|--------|---------|-----------|
| **Unit + integration (fast lane)** | `cargo nextest run --workspace` or `cargo test --workspace` | Unit tests and integration tests that do not need network or live provider. Excludes tests gated by `integration` feature or `[live]` tag. |
| **Extended (slower integration / resilience)** | `cargo nextest run --workspace --profile extended` (or equivalent) | Same as default plus slower integration tests, property tests, resilience tests. Still no live API. |
| **Live-provider (real API)** | `ANTHROPIC_API_KEY=<key> cargo test --workspace --features integration` (or nextest with feature) | Tests that call the real Anthropic API. Tagged `[live]` in test names. Skip if `ANTHROPIC_API_KEY` is unset (CI can skip the lane). |
| **E2E / workflow** | As defined in test plan; may be separate binary or feature-gated | End-to-end workflow tests (e.g. fix a failing test in a fixture repo). May require fixtures and optional `--features e2e` or similar. |

**Summary:**

- **Unit:** Same as integration in Rust; typically `cargo test` in each crate or `cargo nextest run --workspace`.
- **Integration:** In-repo tests that cross crate boundaries; part of fast lane when mocked; part of extended when they are slower or resilience-focused.
- **E2E:** Realistic multi-step runs against fixture repos; may be in `tests/` or a dedicated binary; run explicitly or in extended/CI.
- **Live-provider:** Only with `--features integration` (or equivalent) and `ANTHROPIC_API_KEY` set; never run by default in the fast lane.

---

## 4. Feature flags and env vars that gate tests

| Gate | Effect | When to use |
|------|--------|-------------|
| **`--features integration`** | Enables tests that require network or live provider. | When you want to run live-provider tests; requires `ANTHROPIC_API_KEY`. |
| **`ANTHROPIC_API_KEY`** | If set, live-provider tests run; if unset, CI (and local) can skip or no-op those tests. | Set only when intentionally running live API tests; never commit. |
| **`CLIDO_LOG`** | Log level for the CLI (e.g. `debug`, `trace`). Does not gate tests; useful when debugging test runs. | Optional: `CLIDO_LOG=debug cargo test ...` |
| **nextest profile `extended`** | Includes tests that are slower or flaky (with optional retries). | When running the extended CI lane locally. |

**Important:** The default `cargo test --workspace` or `cargo nextest run --workspace` **must not** run live-provider tests. Those must be behind a feature (e.g. `integration`) and/or excluded by default so that:

- Contributors without API keys can run the full fast lane.
- CI fast lane does not require secrets.

---

## 5. Quick reference

| Goal | Command |
|------|---------|
| Run fast lane (recommended) | `cargo nextest run --workspace` |
| Run fast lane without nextest | `cargo test --workspace` |
| Run with live provider (optional) | `ANTHROPIC_API_KEY=<key> cargo test --workspace --features integration` |
| Run extended profile | `cargo nextest run --workspace --profile extended` |
| Check format and clippy (CI fast lane also runs these) | `cargo fmt --check` then `cargo clippy --workspace -- -D warnings` |
