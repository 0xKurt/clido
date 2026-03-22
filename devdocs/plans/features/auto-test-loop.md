# Feature Plan: Automatic Test Loop

**Status:** Planned
**Crate:** `clido-tools`, `clido-agent`
**Primary files:** `crates/clido-tools/src/test_loop.rs`, `crates/clido-agent/src/sub_agent.rs`
**Estimated effort:** Large (6–8 days)

---

## Problem Statement

"Write code, run tests, fix failures, repeat until green" is the most common coding workflow. Developers do this loop dozens of times per day. Every useful coding agent must support it.

Clido can already perform this workflow: the agent calls Bash to run tests, reads the output, edits the relevant files, and calls Bash again. But this approach has four significant problems:

1. **No structure.** Test output arrives as unstructured text. The model must parse `cargo test` or `pytest` output to understand which tests failed and why. This parsing is token-expensive and error-prone — especially when test output includes ANSI escape codes, long backtraces, or custom test frameworks.
2. **No stop condition.** Nothing prevents the agent from looping indefinitely when a test is genuinely unfixable (e.g., the test itself has a bug, or the fix requires a design change outside the agent's scope). Without a hard iteration limit, a stuck agent burns tokens silently.
3. **No progress visibility.** The user watching the TUI has no indication of whether the agent is on attempt 1 or attempt 7. There is no diff showing which tests were fixed and which remain failing.
4. **No regression detection.** If the agent's fix breaks a previously passing test, there is no mechanism to detect this and roll back. The agent only sees "tests are still failing" without knowing it made things worse.

Claude Code supports an implicit test loop (the model loops via prompting). Cline supports test running but not structured iteration. Neither provides hard stop conditions, regression detection, or per-attempt progress indicators.

---

## Competitive Analysis

| Tool | Test loop support | Stop condition | Progress visibility | Regression detection |
|---|---|---|---|---|
| Claude Code | Implicit via model prompting | None | None | None |
| Cline | Bash + model prompting | None | None | None |
| Aider | `--auto-test` flag | Configurable | Minimal | None |
| GitHub Copilot Workspace | Structured; CI-integrated | CI timeout | CI status | None |
| Clido (proposed) | Structured TestLoop tool | Max iterations + stuck detection | TUI counter + diff | Detects regression per iteration |

Our approach adds the missing layer: structured test output parsing, per-iteration state tracking, stuck-loop detection, and a user-visible progress indicator in the TUI — all driven by a single `TestLoop` tool call.

---

## Our Specific Improvements

### Structured Test Runner Integration

Each backend runs its test command in structured or semi-structured mode and normalizes results to a `TestResult` struct:

```rust
pub struct TestResult {
    pub tests: Vec<TestCase>,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
}

pub struct TestCase {
    pub name: String,
    pub status: TestStatus,
    pub output: Option<String>,   // failure message / backtrace (truncated to 2000 chars)
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

pub enum TestStatus { Passed, Failed, Skipped, Panicked }
```

### Max-Iterations Guard with User-Visible Progress

The loop runs at most `max_iterations` times (default 5, configurable). On each attempt, the TUI updates a dedicated `ChatLine::TestLoopProgress` variant showing the current attempt number, max attempts, and current pass/fail counts.

### Stuck-Loop Detection

After each iteration, compare the set of failing test names with the previous two iterations. If the identical set of tests has failed for 3 consecutive iterations, the loop halts immediately and returns:

```
Stuck: the following tests did not improve after 3 consecutive attempts:
  - auth::tests::test_jwt_expiry
  - auth::tests::test_refresh_token
No further attempts will be made. Manual review required.
```

This is a hard bail condition that cannot be configured away — it prevents unbounded token burn on unfixable tests.

### Regression Detection

Before each fix attempt, snapshot the set of currently passing tests. After applying fixes and re-running, compare the new passing set against the snapshot. If any previously passing test is now failing, the agent reports:

```
Warning: 2 previously passing tests are now failing after this attempt:
  - lib::tests::test_basic_parse  (was passing, now FAILED)
  - lib::tests::test_roundtrip    (was passing, now FAILED)
Rolling back changes and halting loop.
```

Rollback: record file hashes before each edit sub-agent run. On regression detection, restore files to their pre-attempt state using the recorded content. This rollback is implemented as a `SnapshotGuard` that captures `Vec<(PathBuf, Vec<u8>)>` before the sub-agent runs.

### Summary Diff

When the loop completes (success or max iterations), return a summary diff comparing the first run to the final run:

```
Test loop complete after 3 attempts.

Progress:
  Fixed: test_jwt_expiry, test_refresh_token, test_session_expire (3 tests)
  Still failing: test_oauth_flow (1 test)
  Total: 8 passing → 11 passing

Remaining failure:
  auth/tests.rs:142 — test_oauth_flow
  thread 'auth::tests::test_oauth_flow' panicked at 'assertion failed: ...'
```

---

## Test Runner Backends

Auto-detection uses the same marker-file logic as `DiagnosticsTool` (checking for `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`).

### Rust

**Preferred:** `cargo nextest run --message-format libtest-json` (nextest produces NDJSON with structured test results)
**Fallback:** `cargo test -- --format json` (requires nightly) → `cargo test 2>&1` (parse human output with known stable format: `test name ... ok` / `test name ... FAILED`)

Detection: check if `cargo nextest` is available via `which cargo-nextest` or `cargo nextest --version`.

Nextest NDJSON includes `type: "test"`, `event: "started"|"passed"|"failed"`, `name`, `stdout` (for failures). This is the most reliable structured format available in the Rust ecosystem.

### Node.js / TypeScript

Auto-detect runner preference:
1. Check `package.json` scripts for `"test"` key
2. Check for `vitest.config.*` → `vitest run --reporter json`
3. Check for `jest.config.*` → `jest --ci --json`
4. Fallback: `npm test` (unparsed; treat as opaque pass/fail)

Jest `--json` outputs a well-documented JSON format with `testResults[].testFilePath`, `assertionResults[].fullName`, `assertionResults[].status`, `assertionResults[].failureMessages`.

Vitest `--reporter json` outputs a similar structure.

### Python

`pytest -q --tb=short --json-report --json-report-file=/tmp/clido-pytest.json` (requires `pytest-json-report` plugin)

If `pytest-json-report` is not installed: `pytest -q --tb=short` and parse the human-readable summary line: `N failed, N passed, N warning in Xs`. For individual failure details, parse the indented `FAILED test_file.py::TestClass::test_method` lines.

### Go

`go test ./... -v -json` outputs NDJSON with `Action: "pass"|"fail"|"run"`, `Test`, `Output` fields. This format has been stable since Go 1.13. Parse directly.

### Custom

`[tools.test_runner] command = "make test"` — run the command, capture stdout+stderr, attempt to parse as JSON (nextest format, Jest format, pytest-json-report format). If none match, treat exit code 0 as all-pass and non-zero as generic failure with the raw output as the error message.

---

## `TestLoopTool` Design

**File:** `crates/clido-tools/src/test_loop.rs`

```rust
pub struct TestLoopTool {
    config: TestRunnerConfig,
    event_emitter: Arc<dyn EventEmitter>,
}

impl Tool for TestLoopTool {
    fn name(&self) -> &str { "TestLoop" }
    fn description(&self) -> &str {
        "Run tests in a loop, automatically fixing failures until all pass or max iterations reached."
    }
    fn is_read_only(&self) -> bool { false }
    fn schema(&self) -> serde_json::Value { /* see below */ }
    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> Result<String, ClidoError>;
}
```

### Input Schema

```json
{
  "name": "TestLoop",
  "parameters": {
    "max_iterations": {
      "type": "integer",
      "default": 5,
      "description": "Maximum number of fix-and-rerun attempts before halting."
    },
    "target": {
      "type": "string",
      "description": "Optional: specific test name, file, or directory to run. Passed as filter arg to test runner."
    },
    "success_threshold": {
      "type": "number",
      "default": 1.0,
      "description": "Fraction of tests that must pass to consider the loop successful (0.0–1.0). Default 1.0 means all tests must pass."
    }
  }
}
```

### Execution Flow

```
execute(input, ctx):
  1. detect_runner(ctx.workspace)
  2. first_result = run_tests(target)
  3. if first_result.all_pass(threshold): return "All tests passing."
  4. baseline_passing = set of passing test names from first_result
  5. consecutive_same_failures = 0
  6. previous_failing_set = set of failing test names
  7. for attempt in 1..=max_iterations:
       a. emit TestLoopProgress event (attempt, max, passing, failing)
       b. snapshot = snapshot_files(files_referenced_in_failures)
       c. fix_result = run_fix_sub_agent(failing_tests, ctx)
       d. new_result = run_tests(target)
       e. if regression_detected(baseline_passing, new_result):
            restore_snapshot(snapshot)
            return regression_error(new_result)
       f. if new_result.all_pass(threshold): return success_summary(...)
       g. new_failing_set = set of failing test names from new_result
       h. if new_failing_set == previous_failing_set:
            consecutive_same_failures += 1
          else:
            consecutive_same_failures = 0
            previous_failing_set = new_failing_set
       i. if consecutive_same_failures >= 3: return stuck_error(new_failing_set)
  8. return max_iterations_reached_summary(first_result, new_result)
```

### Sub-Agent for Fixing

The sub-agent is a new `SubAgent` (in `crates/clido-agent/src/sub_agent.rs`) that runs a constrained agent loop with:
- **System prompt:** Focused entirely on fixing failing tests. Includes the failing test names, their error output (truncated), and the relevant source file paths.
- **Allowed tools:** `Read`, `Write`, `Edit`, `Glob`, `Grep`, `Bash` only. `TestLoop` is explicitly excluded to prevent recursion.
- **Max turns:** 10 (configurable via `[tools.test_runner] sub_agent_max_turns = 10`).
- **No user interaction:** Sub-agent runs to completion without waiting for user input.

```rust
pub struct SubAgent {
    pub agent_config: AgentConfig,
    pub allowed_tools: Vec<String>,
    pub system_prompt_suffix: String,
    pub max_turns: u32,
}

impl SubAgent {
    pub async fn run(&self, user_message: String) -> Result<SubAgentResult, ClidoError>;
}
```

The sub-agent receives a user message of the form:
```
Fix the following failing tests. Do not run tests yourself; just fix the code.

Failing tests:
- auth::tests::test_jwt_expiry
  File: src/auth/tests.rs:88
  Error: assertion `left == right` failed
         left: Expired
        right: Valid

- auth::tests::test_refresh_token
  File: src/auth/tests.rs:142
  Error: called `Result::unwrap()` on an `Err` value: TokenExpired
```

---

## Agent-Loop Integration

**File:** `crates/clido-agent/src/agent_loop.rs`

The `AgentLoop` gains an optional `sub_agent_context: Option<SubAgentContext>` field that is set when `TestLoopTool` spawns a sub-agent. This context limits the available tools and injects the constrained system prompt. The existing `run_next_turn` method respects these constraints when `sub_agent_context` is `Some`.

The `EventEmitter` trait (already in `clido-cli/src/`) gains a new event variant:

```rust
pub enum AgentEvent {
    // ... existing variants ...
    TestLoopProgress {
        attempt: u32,
        max_attempts: u32,
        passing: u32,
        failing: u32,
        newly_fixed: Vec<String>,
    },
    TestLoopComplete {
        total_attempts: u32,
        final_passing: u32,
        final_failing: u32,
        fixed_tests: Vec<String>,
        still_failing: Vec<String>,
    },
    TestLoopStuck {
        stuck_tests: Vec<String>,
        attempts_made: u32,
    },
    TestLoopRegression {
        regressed_tests: Vec<String>,
    },
}
```

---

## TUI Changes

### New `ChatLine` Variant

**File:** `crates/clido-cli/src/tui.rs`

```rust
pub enum ChatLine {
    // ... existing variants ...
    TestLoopProgress {
        attempt: u32,
        max_attempts: u32,
        passing: u32,
        failing: u32,
    },
    TestLoopComplete {
        total_attempts: u32,
        fixed_tests: Vec<String>,
        still_failing: Vec<String>,
    },
    TestLoopStuck {
        stuck_tests: Vec<String>,
    },
}
```

`TestLoopProgress` renders as a single updating line (overwrite-in-place using ratatui's `StatefulWidget` or simply a distinct line per attempt):
```
[Test Loop] Attempt 2/5 — 8 passing, 3 failing
```

Using ratatui styles: attempt counter in `Color::Cyan`, "passing" count in `Color::Green`, "failing" count in `Color::Red`.

`TestLoopComplete` renders a summary block:
```
[Test Loop] Complete in 3 attempts
  Fixed: test_jwt_expiry, test_refresh_token, test_session_expire
  Still failing: test_oauth_flow
```

`TestLoopStuck` renders in yellow:
```
[Test Loop] Stuck after 3 attempts — manual review required
  Unchanged failures: test_jwt_expiry, test_refresh_token
```

### New `/testloop` Slash Command

Add `/testloop` to `SLASH_COMMANDS` in `tui.rs`. Usage:

```
/testloop [target] [max=N]
```

Examples:
```
/testloop
/testloop auth max=10
/testloop src/auth/tests.rs
```

When invoked, this injects a user message of `"Run test loop for [target]"` into the agent, which then calls `TestLoopTool`.

---

## CLI Surface

**New flag:** `--test-loop` modifier flag for the `run` subcommand.

```
clido run --test-loop "fix the auth module tests"
clido run --test-loop --max-iterations 10 "fix all failing tests"
```

When `--test-loop` is present, the system prompt is augmented with test-loop-focused instructions and `TestLoopTool` is pre-authorized.

**File:** `crates/clido-cli/src/cli.rs`

Add to `RunArgs`:
```rust
#[arg(long)]
pub test_loop: bool,

#[arg(long, default_value = "5")]
pub max_iterations: u32,
```

---

## Config Schema

Full addition to `~/.clido/config.toml`:

```toml
[tools.test_runner]
# Test command override. If empty, auto-detect from workspace.
command = ""

# Maximum iterations before halting (can be overridden per TestLoop call)
max_iterations = 5

# Timeout per test run in seconds
timeout_secs = 120

# Maximum turns the fix sub-agent can take per iteration
sub_agent_max_turns = 10

# Fraction of tests that must pass to consider loop successful (0.0–1.0)
success_threshold = 1.0

# Number of consecutive identical failure sets before declaring stuck
stuck_threshold = 3
```

Add `TestRunnerConfig` struct to `AgentConfig` in `crates/clido-core/src/config_loader.rs`.

---

## Error Variants

In `crates/clido-core/src/error.rs`, add:

```rust
pub enum ClidoError {
    // ... existing ...
    TestLoopStuck { stuck_tests: Vec<String>, attempts: u32 },
    TestLoopRegression { regressed_tests: Vec<String> },
    TestLoopMaxIterations { attempts: u32, still_failing: Vec<String> },
    TestRunnerTimeout { command: String, timeout_secs: u64 },
    TestRunnerNotFound { workspace: PathBuf },
    TestOutputParseError { runner: String, raw: String },
}
```

---

## File Structure

```
crates/clido-tools/src/test_loop/
    mod.rs          — TestLoopTool, loop orchestration
    runner.rs       — TestRunner trait, run_tests(), detect_runner()
    backends/
        mod.rs
        nextest.rs  — cargo nextest NDJSON parser
        cargo.rs    — cargo test human-output parser
        jest.rs     — Jest --json parser
        vitest.rs   — vitest --reporter json parser
        pytest.rs   — pytest-json-report + human fallback
        go.rs       — go test -json NDJSON parser
        custom.rs   — custom command with multi-format detection
    snapshot.rs     — SnapshotGuard: capture/restore file contents
    result.rs       — TestResult, TestCase, TestStatus structs

crates/clido-agent/src/
    sub_agent.rs    — SubAgent struct, constrained agent loop runner
```

---

## Test Plan

All tests in `crates/clido-tools/tests/test_loop_tests.rs` and unit tests in backends.

```rust
// Backend parser tests
fn test_nextest_parses_pass_event()
fn test_nextest_parses_fail_event_with_output()
fn test_nextest_parses_mixed_results()
fn test_cargo_test_parses_human_output_pass()
fn test_cargo_test_parses_human_output_fail()
fn test_cargo_test_parses_panic_output()
fn test_jest_parses_json_output()
fn test_jest_parses_empty_test_suite()
fn test_vitest_parses_json_output()
fn test_pytest_parses_json_report()
fn test_pytest_parses_human_fallback_summary()
fn test_go_test_parses_ndjson()
fn test_custom_runner_exit_zero_is_pass()
fn test_custom_runner_exit_nonzero_is_fail()

// Auto-detection tests
fn test_detect_nextest_when_available()
fn test_detect_cargo_fallback_when_nextest_absent()
fn test_detect_jest_from_package_json()
fn test_detect_vitest_from_config_file()
fn test_detect_pytest_from_pyproject_toml()
fn test_detect_go_from_go_mod()

// Loop orchestration tests (using mock runner)
fn test_loop_exits_on_all_pass_first_attempt()
fn test_loop_runs_sub_agent_and_retries()
fn test_loop_halts_at_max_iterations()
fn test_loop_halts_on_stuck_detection_after_3_identical_failures()
fn test_loop_detects_regression_and_rolls_back()
fn test_loop_success_threshold_partial_pass()
fn test_loop_respects_target_filter()

// Snapshot/rollback tests
fn test_snapshot_captures_file_contents()
fn test_snapshot_restores_on_regression()
fn test_snapshot_handles_new_files_created_by_sub_agent()

// Sub-agent isolation tests
fn test_sub_agent_cannot_call_test_loop_recursively()
fn test_sub_agent_has_limited_tool_set()
fn test_sub_agent_respects_max_turns()

// Integration test (mock test runner)
fn integration_test_full_loop_mock_runner()
fn integration_test_loop_with_regression_mock()
fn integration_test_stuck_detection_mock()
```

---

## Docs Pages to Update / Create

- **Create** `docs/guide/test-loop.md` — user-facing guide. Covers the TestLoop tool, `/testloop` slash command, `--test-loop` CLI flag, supported test runners, configuration, and interpreting the progress output. Include a worked example showing a 3-iteration loop fixing a borrow-checker error.
- **Update** `docs/guide/running-prompts.md` — add section "Running a test loop" with `clido run --test-loop` examples.
- **Update** `docs/reference/cli.md` — document `--test-loop` and `--max-iterations` flags on the `run` subcommand.
- **Update** `docs/reference/configuration.md` — document the `[tools.test_runner]` config section.
- **Update** `docs/guide/slash-commands.md` — add `/testloop` with usage examples.
- **Update** `docs/developer/adding-tools.md` — add section on implementing backend parsers using the `BackendTrait` pattern shared with `DiagnosticsTool`.

---

## Definition of Done

- [ ] `TestLoopTool` struct implements `Tool` trait with correct `name()`, `is_read_only() = false`, `schema()`, `execute()`
- [ ] All seven backend parsers implemented: `nextest.rs`, `cargo.rs`, `jest.rs`, `vitest.rs`, `pytest.rs`, `go.rs`, `custom.rs`
- [ ] Auto-detection logic selects the correct backend for each supported language
- [ ] `max_iterations` guard halts the loop and returns a clear error after N attempts
- [ ] Stuck-loop detection halts after 3 consecutive identical failure sets with named test list in error
- [ ] Regression detection identifies newly-failing tests and triggers rollback via `SnapshotGuard`
- [ ] `SnapshotGuard` captures file contents before each sub-agent run and restores them on regression
- [ ] Sub-agent runs with restricted tool set (no TestLoop recursion); enforced at agent loop level
- [ ] Sub-agent respects `sub_agent_max_turns` limit
- [ ] `TestLoopProgress`, `TestLoopComplete`, `TestLoopStuck` `ChatLine` variants implemented and rendered in TUI with correct ratatui styles
- [ ] `/testloop` slash command added to `SLASH_COMMANDS` in `tui.rs` and handled correctly
- [ ] `--test-loop` and `--max-iterations` flags added to `RunArgs` in `cli.rs`
- [ ] `TestRunnerConfig` deserialized from `[tools.test_runner]` TOML section in `AgentConfig`
- [ ] All six error variants added to `ClidoError` and handled in error formatting
- [ ] All 40+ test functions listed above pass
- [ ] `AgentEvent::TestLoopProgress`, `TestLoopComplete`, `TestLoopStuck`, `TestLoopRegression` variants added to `EventEmitter`
- [ ] `docs/guide/test-loop.md` created with worked example
- [ ] `docs/reference/cli.md` updated with new flags
- [ ] `docs/reference/configuration.md` updated with `[tools.test_runner]` section
- [ ] `cargo clippy -- -D warnings` passes on `clido-tools` and `clido-agent`
