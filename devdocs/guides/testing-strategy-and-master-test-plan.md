# Testing Strategy and Master Test Plan

## Purpose

This document defines the full testing strategy for Clido.

It is broader than the milestone-level testing notes in `devdocs/plans/development-plan.md`. The roadmap describes what should be tested during implementation. This guide defines how the entire system should be validated over time, across unit, integration, end-to-end, security, performance, resilience, release, and operational concerns.

The goal is simple:
- detect defects early
- prevent regressions
- make failures easy to diagnose
- prove that Clido is useful, safe, and reliable in real workflows

## Testing Philosophy

### Test outcomes, not optimism

Do not assume a feature works because:
- the code compiles
- a happy-path demo succeeded
- one manual run looked correct

A feature is only trustworthy when its expected behavior, edge cases, and failure modes are covered.

### Test the system at multiple levels

No single test type is enough.

Clido needs:
- unit tests for correctness of small logic
- integration tests for crate boundaries and tool execution
- end-to-end tests for realistic agent behavior
- regression tests for previously fixed bugs
- security tests for dangerous paths
- performance tests for latency and scale
- resilience tests for crashes, retries, malformed inputs, and partial failure

### Prefer deterministic tests where possible

- Control randomness.
- Freeze time when practical.
- Mock external services for most automated tests.
- Reserve live-provider tests for explicit integration lanes.
- Keep failure reproduction simple.

### Use real behavior where it matters

Mocking is useful, but not enough.

Some parts of Clido must be validated against real conditions:
- real filesystem behavior
- real subprocess execution
- real streaming behavior
- real provider API contracts
- real repository-sized workloads

## Quality Goals

The test plan should prove that Clido:
- produces correct outputs for its core workflows
- handles tool and provider failures safely
- preserves session state and resumes correctly
- respects permissions and safety boundaries
- performs well enough for interactive use
- remains stable as features such as memory, subagents, MCP, and planner are added

## Test Taxonomy

### 1. Unit tests

Unit tests verify isolated logic with minimal dependencies.

Use unit tests for:
- data structures
- parsing and serialization
- token counting helpers
- config merging
- permission matching
- schema validation
- retry policy logic
- path validation rules
- diff generation logic
- planner graph validation

### 2. Integration tests

Integration tests verify interaction between multiple components inside Clido.

Use integration tests for:
- provider + agent loop interaction with mocked responses
- tool registry + tool execution
- session persistence and replay
- context builder + compaction
- permission checker + CLI mode behavior
- memory store + retrieval
- subagent lifecycle

### 3. End-to-end tests

End-to-end tests validate Clido as a user would experience it.

Use end-to-end tests for:
- CLI invocation
- streaming output
- multi-step tool use
- file edits on fixture repositories
- session interruption and resume
- real task completion across several turns

### 4. Regression tests

Regression tests capture bugs that have already happened.

Every important defect should leave behind:
- a minimal reproducible fixture
- an automated test that fails before the fix
- a clear name that explains the original bug

### 5. Security tests

Security tests validate that Clido does not overreach or leak sensitive data.

Use security tests for:
- secret handling
- path traversal denial
- sandbox restrictions
- dangerous Bash behavior
- permission gating
- config and audit data exposure

### 6. Performance tests

Performance tests validate interactive responsiveness and scalability.

Use them for:
- startup time
- context building cost
- file tool throughput
- search performance
- memory retrieval overhead
- indexing performance
- planner overhead

### 7. Resilience tests

Resilience tests validate Clido under bad conditions.

Use them for:
- provider timeouts
- 429s and 5xx retries
- malformed tool JSON
- interrupted sessions
- flaky subprocesses
- partial streaming responses
- corrupted session lines

## Test Environment Strategy

### Test environments

Maintain at least these environments:

1. Local developer environment
- fast feedback
- mostly unit and integration tests
- optional targeted end-to-end runs

2. CI fast lane
- formatting
- linting
- unit tests
- deterministic integration tests
- no secrets required

3. CI extended lane
- slower integration suites
- fixture repositories
- concurrency tests
- property tests
- performance smoke checks

4. CI live-provider lane
- runs only when secrets are available
- validates provider contract compatibility
- should be isolated and rate-limited

5. Pre-release validation lane
- full regression suite
- fixture repo task runs
- benchmark comparison
- packaging smoke tests
- resume / recovery checks

### Test data sources

Use a mix of:
- small synthetic fixtures for precision
- medium repositories for realistic workflows
- large repositories for performance and search behavior
- intentionally broken fixtures for negative tests

## Repository Test Layout

Recommended structure:

```text
tests/
├── unit/
├── integration/
├── e2e/
├── regression/
├── security/
├── performance/
├── resilience/
└── fixtures/
    ├── sample-project/
    ├── broken-project/
    ├── large-project/
    ├── secret-fixtures/
    ├── session-fixtures/
    └── planner-fixtures/
```

Recommended fixture rules:
- fixtures should be small unless size is the point of the test
- every fixture should have a clear purpose
- broken fixtures should be intentionally broken in one obvious way
- fixture names should explain what they are testing

## Rust Testing Toolchain

Use the following tools consistently across all crates:

| Tool | Purpose |
|------|---------|
| `cargo test` | standard unit and integration tests |
| `cargo nextest` | faster parallel test runner; preferred in CI |
| `cargo tarpaulin` | code coverage measurement |
| `wiremock-rs` | HTTP mock server for provider tests |
| `tempfile` | temporary directories and files in tests |
| `proptest` | property-based testing |
| `cargo fuzz` | structured fuzzing via libFuzzer |
| `httpmock` | simpler HTTP mocking where wiremock is overkill |

All tests use `#[tokio::test]` for async. Synchronous tests may use standard `#[test]`.

Tests that require network access or API keys are gated behind `#[cfg(feature = "integration")]` and excluded from the fast lane.

## Coverage Targets

These targets supersede those in `development-plan.md` Phase 9.1.1. Per-release floors are defined in Phase 5.4.2 of the roadmap. The table below shows the final (V4 / production) targets:

| Crate | V1 floor | V2 floor | V3 floor | V4 / final target |
|-------|----------|----------|----------|-------------------|
| `clido-core` | 85% | 90% | 92% | ≥ 95% |
| `clido-tools` | 80% | 85% | 87% | ≥ 90% |
| `clido-providers` | 75% | 80% | 85% | ≥ 85% |
| `clido-context` | 70% | 75% | 80% | ≥ 85% |
| `clido-storage` | 75% | 80% | 82% | ≥ 85% |
| `clido-memory` *(V3+)* | — | — | 70% | ≥ 85% |
| `clido-agent` | 70% | 75% | 80% | ≥ 80% |
| `clido-planner` *(V4+)* | — | — | — | ≥ 80% |
| `clido-index` *(V3+)* | — | — | 70% | ≥ 75% |
| Workspace overall | 70% | 75% | 78% | ≥ 82% |

Measure with `cargo tarpaulin --workspace --all-features`. Track coverage in CI and treat regressions below the per-release floor as blocking.

## Crate-by-Crate Test Plan

### `clido-core`

**Goal:** Validate shared types and foundational invariants. Target: ≥ 95% coverage.

**Test categories:**
- JSON round-trip tests for `Message`, `ContentBlock`, `Usage`, `ModelResponse`
- enum serialization and deserialization for all variants
- error conversion and `From` impl tests
- telemetry event schema tests
- property tests using `proptest` for serialization stability

**Critical cases:**
- `ToolUse` and `ToolResult` blocks serialize and deserialize without data loss
- malformed JSON fails cleanly with a useful error
- optional fields round-trip correctly and remain backward-compatible when absent

### `clido-tools`

**Goal:** Validate each tool independently and through the registry. Target: ≥ 90% coverage.

**Tool-specific coverage:**

#### `Read`

Test:
- normal file read
- offset and limit reads
- correct line-prefix format (`     N→` with right-aligned line numbers)
- nonexistent path returns `is_error: true` with cwd in message
- directory path returns `EISDIR` error
- unreadable file (permission denied)
- large file truncation if enabled
- repeated-read dedup behavior (V1.5 and later)

#### `Write`

Test:
- write new file
- overwrite existing file
- nested parent directory creation
- write failure on permission error
- secret detection warning path (V1.5 and later)
- root path restriction (if configured)

#### `Edit`

Test:
- replace first occurrence
- replace all occurrences (`replace_all: true`)
- string not found: exact error format `<tool_use_error>String to replace not found...`
- deletion via empty `new_string`
- file changed between Read and Edit
- diff metadata generation in session record

#### `Glob`

Test:
- simple single-level pattern match
- recursive pattern match with `**`
- ignored directories behavior (`.git`, `node_modules`, etc.)
- no matches: empty result, not an error
- invalid pattern

#### `Grep`

Test:
- content mode with context lines
- file list mode (`output_mode: files_with_matches`)
- count mode
- case-insensitive flag
- unknown parameter rejected with `InputValidationError`
- large directory search does not timeout

#### `Bash`

Test:
- successful command: stdout captured, `is_error: false`
- non-zero exit: exit code and stderr in content, `is_error: true`
- timeout: correct error message
- combined stdout and stderr capture
- sensitive env vars stripped before subprocess spawn
- cwd set correctly
- sandbox denial behavior (V2 and later)

#### `MemoryTool` *(V3 and later)*

Test:
- `action: save`
- `action: search`
- `action: list`
- malformed or missing action field
- storage backend failures

**Registry and schema tests:**
- tool registration and name lookup
- duplicate tool name rejected or replaced consistently
- `schemas()` returns valid JSON Schema objects
- schema validation rejects invalid input before execution
- `is_read_only()` returns correct values for all tools

### `clido-providers`

**Goal:** Validate request building, response parsing, retries, and provider normalization. Target: ≥ 85% coverage.

**Test categories:**
- request body serialization verified against `wiremock-rs` mock servers
- tool schema mapping for each provider format
- streaming SSE event parsing
- stop reason mapping
- usage token accounting
- retry behavior for 429 with and without `Retry-After` header
- retry behavior for 5xx (up to 3 attempts)
- no retry for non-retryable 4xx (400, 401, 403)
- malformed JSON in provider response

**Provider-specific coverage:**

#### Anthropic

Test:
- messages API request shape matches Anthropic format
- tool definitions included in correct format
- tool results in user role with `tool_use_id`
- SSE stream parsing: `content_block_delta`, `message_delta`
- partial input JSON assembled correctly before execution

#### OpenAI-compatible providers

Test:
- `tool_calls` array in assistant message
- tool results as `tool` role messages with `tool_call_id`
- function parameter schema compatibility
- provider-specific headers (e.g., `HTTP-Referer` for OpenRouter)

#### Local provider fallback *(V3 and later)*

If text-to-tool parsing is implemented, test:
- valid tool call extracted from natural language output
- malformed JSON-like output falls back gracefully
- incorrect tool name produces `is_error: true` result

### `clido-context`

**Goal:** Validate correctness and efficiency of prompt construction. Target: ≥ 85% coverage.

**Test categories:**
- token counting accuracy (within ±10% of actual model tokenizer)
- system prompt injection as first message or API `system` field
- `CLIDO.md` and `CLAUDE.md` project instruction loading
- tool guidance block generation for registered tools
- context size threshold detection
- compaction trigger when approaching `max_tokens * threshold`
- compacted history replaces old messages with summary
- unchanged message retention after compaction
- memory injection into system prompt (V3 and later)

**Critical cases:**
- long conversations compact rather than exceeding the context limit
- compacted summary retains enough context for the model to continue meaningfully
- recent tool results are never compacted away
- duplicated reads of unchanged files do not appear twice in context

### `clido-storage`

**Goal:** Validate persistence, replay, and session recovery. Target: ≥ 85% coverage.

**Test categories:**
- session JSONL file created on first write
- each session line appended immediately (no buffering)
- session fully reconstructed from JSONL on `--resume`
- session fork creates independent copy
- malformed JSONL line: rest of session is still readable
- missing session: clear error, not a panic
- concurrent session IDs produce isolated files

**Critical cases:**
- process killed mid-turn: session file has all turns up to the interruption
- corrupted line does not silently truncate or corrupt the rest of the session
- session metadata (ID, timestamps, turn count) is consistent after multi-turn runs

### `clido-memory` *(V3 and later)*

**Goal:** Validate both in-session and persistent memory behavior. Target: ≥ 85% coverage (V3 floor: 70%).

**Test categories:**
- in-memory store: save, search by keyword, list recent
- SQLite store: save, FTS5 search, recent retrieval
- tag-based filtering
- duplicate inserts handled gracefully
- deletion removes from both tables
- memory injection into new session context
- relevance ranking sanity (high-score results appear first)

**Hybrid retrieval (new — required for V3):**
- FTS5 search returns correct keyword matches from a fixture DB
- Vector search (`sqlite-vec`) returns semantically similar entries (use a controlled pair of near-identical sentences, verify cosine similarity > threshold)
- Reciprocal Rank Fusion merges FTS5 and vector results correctly (mock both result lists, verify final ordering is correct)
- Query with no matches in either index returns an empty result set, not an error
- Hybrid search does not regress latency beyond 50 ms for a 5,000-entry DB (benchmark with `criterion`)

**Embedding engine (`fastembed-rs`) (new — required for V3):**
- `EmbeddingEngine::embed()` returns a 384-dimension vector for a known input
- Offline mode: if ONNX model cache is absent and `offline = true`, `embed()` returns `Err`, not a silent zero vector
- `spawn_blocking` wrapper: calling `embed()` from an async context does not block the Tokio runtime (integration test with a 10ms sleep probe on the runtime thread)

**Per-session write cap (new — required for V3):**
- Writing 51 entries with `max_writes_per_session = 50` → 51st write returns `is_error: true` with the cap message; 50 entries in DB
- Write counter resets at the start of a new session (counter is session-scoped, not persisted)

**Deduplication (new — required for V3):**
- Saving an entry with cosine similarity ≥ 0.95 to an existing entry → no duplicate inserted, existing entry's `created_at` updated
- Saving an entry with similarity < 0.95 → new entry inserted

**Capacity and eviction:**
- Insert 10,001 entries with `max_entries = 10,000` → DB contains exactly 10,000 entries; oldest entry evicted
- `clido memory prune --keep 100` → exactly 100 entries remain

**Risk-focused tests:**
- irrelevant or stale memory does not crowd out recent tool results in context
- large memory stores (1,000+ entries) remain performant; search latency < 50 ms on a 5,000-entry DB
- memory DB survives process restart and reads correctly on re-open
- schema migration (DB version 0 → 1) applies cleanly on re-open; no data loss (see Phase 9.7.2)

### `clido-planner` *(V4 and later)*

**Goal:** Validate task graph correctness and safe fallback behavior. Target: ≥ 80% coverage.

**Test categories:**
- task schema validation
- cycle detection in dependency graph
- topological ordering of valid graph
- invalid graph produces fallback, not a panic
- planner model response parsed correctly
- DAG executor resolves nodes in dependency order
- partial node failure: remaining independent nodes still run
- fallback to reactive loop when planner graph is invalid or absent

**Key rule:** The planner must never be allowed to silently degrade baseline reliability. Test the reactive-loop fallback path as carefully as the planner itself.

### `clido-index` *(V3 and later)*

**Goal:** Validate index build, update, and search quality. Target: ≥ 75% coverage.

Test:
- full index build on fixture repository
- incremental update adds new files
- incremental update removes deleted files
- symbol extraction produces correct results for known source files
- search returns correct files for known queries
- stale index is detected and invalidated
- large-repo build completes within a reasonable time bound

### `clido-agent`

**Goal:** Validate the core agent loop, execution ordering, and recovery. Target: ≥ 80% coverage.

**Test categories:**
- user message is added to history correctly
- model text response becomes assistant history
- tool calls execute in the order returned by the model
- read-only tool batches execute concurrently via `join_all`
- state-changing tools execute sequentially
- `max_turns` enforcement stops the loop and returns a clear result
- `max_budget_usd` enforcement stops the loop when cost exceeds the limit
- permission check: `Allow` → tool runs; `Deny` → `is_error: true` returned to model
- plan-mode: Write, Edit, Bash produce denied tool results
- failed tool result passed back into history as a user message
- graceful Ctrl-C: session flushed before exit
- subagent launched and result returned to parent (V3 and later)

**Critical named integration scenarios:**

| Scenario | Description |
|----------|-------------|
| Text-only response | Model returns text with no tool calls; result is printed |
| Single tool call | Model calls Read; file content returned; model prints summary |
| Parallel reads | Model calls Read × 3; all execute concurrently; results combined |
| Mixed tools | Model calls Read and Edit; Read first, Edit sequential |
| Edit failure and retry | Edit fails; model issues corrected Edit; file updated |
| Provider 429 retry | Provider returns 429; retry succeeds; session continues |
| Provider failure exhaustion | All retries fail; session ends with clear error |
| User-denied tool | Permission prompt returns Deny; `is_error: true` in history |
| Plan-mode denial | Bash blocked in plan mode; model continues with read-only tools |
| Resume after interruption | Session killed mid-turn; `--resume` reconstructs and continues |
| Permission prompt serialized | Two concurrent parallel read tools both trigger `AskUser`; verify exactly one prompt at a time, no garbled output, no deadlock within 5s |
| Subagent permission shared | Two parallel subagents each require `AskUser`; verify serialized prompts, no deadlock |
| Compaction failure fallback | Summarization call returns HTTP 500; verify compaction is skipped, agent continues, `warn!` event emitted |
| Compaction hard-limit | Context exceeds model's hard token limit and summarization fails; verify user-visible error and halted turn (no panic) |
| Unsupported provider V1 | Profile specifies `openrouter` in V1 binary; verify `ClidoError::Config` not runtime panic |
| Model tool-use warning | Provider factory gets a model known not to support tool use; verify startup warning emitted, agent still starts |
| Offline mode guard | `offline = true` with non-localhost provider URL; verify startup error |
| Offline mode fastembed | `offline = true`, ONNX cache absent; verify `embed()` returns `Err` with clear message |
| Windows BashTool degraded | No POSIX shell found on Windows (mocked); verify `is_error: true` with platform message |
| LRU mtime+hash | File touched (mtime changes, content unchanged); verify cache returns cached content, mtime updated, no disk re-read |
| LRU false-positive | Two edits within same mtime granularity; verify hash catches the change |
| Schema migration session | Load V0 session fixture with V1 reader; verify migration applied, all fields parse |
| Schema migration DB | Open V0 memory DB with V1 binary; verify schema migrated, no data loss |
| pricing.toml stale | Pricing file >90 days old; verify staleness warning emitted at startup |
| pricing.toml missing model | Request a model not in pricing table; verify warn+fallback, no panic |

## CLI and UX Test Plan

### CLI behavior

Test:
- `--help`
- `-p` / `--print`
- `--output-format text`
- `--output-format json`
- `--output-format stream-json`
- `--resume`
- `list-sessions`
- `show-session`
- `doctor`
- invalid flag combinations

### Interactive behavior

If REPL exists, test:
- prompt display
- multiline conversation continuity
- exit behavior
- Ctrl-C handling
- resumed session visibility

### Output contract tests

For machine-readable modes, assert:
- valid JSON shape
- newline-delimited JSON in stream mode
- stable field names
- proper `is_error` semantics
- predictable final result structure

## End-to-End Workflow Suite

Maintain a curated set of real workflows.

### Core workflows

Clido should be tested against tasks such as:
- list files in a repository
- explain how a module works
- find all usages of a symbol
- update a configuration value
- add a small new file
- fix a syntax error
- fix a failing unit test
- audit a small repository for obvious issues
- resume an interrupted task

### Advanced workflows

As features are added, expand with:
- run in plan mode and produce a safe read-only analysis
- use subagents to inspect multiple files
- retrieve relevant memory in a new session
- use MCP-provided tools
- use indexed search on larger repositories
- run planner-guided execution on large tasks

### Evaluation dimensions

For each workflow, record:
- success or failure
- number of turns
- tool calls used
- total cost
- duration
- whether edits were correct
- whether safety rules were respected

## Security Test Plan

### File access controls

Test:
- path traversal attempts
- absolute paths outside the allowed root
- symlink escapes
- writes outside allowed workspace

### Secret safety

Test:
- detection of known secret patterns
- no secret logging in stdout, stderr, session, or telemetry
- environment stripping for subprocesses

### Permission model

Test:
- default deny or prompt behavior
- allowlist and denylist precedence
- session-scoped approvals
- non-interactive denial behavior

### Shell sandboxing

If sandboxing exists, test:
- forbidden network access
- denied writes outside allowed areas
- blocked access to user secret directories
- failure messaging remains clear

## Resilience and Failure-Injection Plan

### Provider failures

Inject:
- timeout
- connection reset
- 429 with and without retry-after
- 500, 502, 503
- malformed JSON
- truncated streaming events

Verify:
- retries follow policy
- retry exhaustion is reported clearly
- session state remains consistent

### Tool failures

Inject:
- panic inside tool execution
- invalid schema input
- subprocess timeout
- filesystem permission error
- partial file write failure

Verify:
- failure returns `is_error: true`
- the agent can continue when appropriate
- logs and session state remain coherent

### Storage failures

Inject:
- disk full simulation where feasible
- write permissions denied
- malformed JSONL line
- interrupted session file flush

Verify:
- recovery paths are clear
- errors are explicit
- silent corruption does not occur

## Property-Based and Fuzz Testing

Use property tests for:
- message serialization round-trips
- random tool input validation edge cases
- task graph validation
- path normalization rules
- compaction invariants
- edit operation behavior under arbitrary strings

Use fuzzing for:
- streamed provider event parsing
- tool input JSON parsing
- session file parsing
- planner output parsing
- MCP message parsing if implemented

## Performance and Benchmark Plan

### Performance goals

Track and defend:
- startup latency
- time to first streamed token
- average duration for common tasks
- large-file read behavior
- search performance in medium and large repos
- memory retrieval overhead
- compaction overhead

### Benchmark categories

- microbenchmarks for pure helpers
- crate-level benchmarks for token counting and search
- workflow benchmarks for real tasks
- concurrency benchmarks for read-heavy turns

### Performance regression rules

- keep benchmark history for important metrics
- define alert thresholds for regression
- investigate meaningful slowdowns before release

## Manual Testing Plan

Automated testing is necessary, but some behaviors still need intentional human review.

### Manual test areas

- readability and usefulness of final responses
- streaming UX quality
- permission prompt clarity
- doctor-command usefulness
- session resume usability
- output ergonomics in real terminals
- safety messaging for denied operations

### Manual scenario checklist

Before major releases, manually test:
- a fresh install
- initial configuration
- a basic repository exploration task
- a safe edit task
- an interrupted session
- a denied tool flow
- a non-interactive JSON-output run

## CI Strategy

### Fast lane

Run on every push:
- format checks
- lints
- unit tests
- deterministic integration tests

### Extended lane

Run on main branch and release branches:
- broader integration suite
- resilience tests
- property tests
- performance smoke checks

### Live-provider lane

Run on schedule or protected branches:
- real API compatibility tests
- streaming contract verification
- provider cost and usage sanity checks

### Release gate

Require before release:
- all critical tests green
- no known flaky critical tests
- benchmark regressions reviewed
- security checks green
- packaging smoke tests green

## Release Validation Checklist

Before tagging a release:
- core workflow suite passes
- resume and recovery suite passes
- permission and security suite passes
- output contract tests pass
- benchmark regressions are acceptable
- documentation reflects actual behavior
- packaging and install paths work on target platforms

## Metrics and Ongoing Quality Signals

Track over time:
- test pass rate
- flaky test rate
- median task duration
- median tool-call count per workflow
- provider retry rate
- crash rate
- session resume success rate
- benchmark trend lines
- top recurring regression classes

## Best Practices for Writing Tests

- name tests after behavior, not implementation details
- keep fixtures minimal and explicit
- avoid over-mocking core business logic
- assert meaningful outputs
- keep failure messages readable
- isolate non-determinism
- fix flaky tests quickly
- add regression tests immediately after bug fixes

## Testing Priorities by Release

### V1

Focus on:
- core tools
- agent loop correctness
- context compaction (including failure fallback)
- permission model (including serialized `AskUser` gate, no deadlock)
- resume and recovery (stale-file detection)
- end-to-end repository workflows
- provider error handling (unsupported provider, missing API key)
- `clido doctor` V1 checks
- Windows BashTool degradation path (if CI runs on Windows)
- session schema versioning V0 → V1 migration
- `pricing.toml` staleness warning
- coverage floor ≥ 70% workspace-wide (see Phase 5.4.2)

### V1.5

Add focus on:
- cost tracking (per-session, prompt caching metrics)
- bounded concurrency (semaphore, no starvation)
- JSON output contracts (exit status taxonomy)
- secret detection
- operator diagnostics (`clido doctor` MCP checks)
- pricing.toml `update-pricing` command (offline-safe)

### V2

Add focus on:
- multi-provider compatibility (OpenAI, OpenRouter, Alibaba)
- offline mode (`offline = true` enforcement, fastembed cache absent)
- model tool-use compatibility warning
- sandboxing
- audit behavior
- packaging (cross-platform builds, Windows Tier 2 validation)
- startup and throughput performance
- prompt caching metrics and cost accuracy

### V3

Add focus on:
- memory relevance (hybrid FTS5 + vector search, relevance ranking)
- embedding engine (fastembed ONNX, offline path, no runtime blocking)
- per-session write cap
- semantic deduplication
- memory DB schema migrations
- subagent correctness (budget propagation, permission gate sharing)
- MCP interoperability
- repository indexing quality

### V4

Add focus on:
- planner quality versus baseline loop
- planner checkpoint/rollback (partial execution recovery)
- fallback behavior (planner failure → reactive loop)
- orchestration complexity versus measurable benefit
- self-improvement loop correctness (instruction update proposals, safety guardrails)

## Final Rule

If Clido cannot be tested confidently, it is not ready to be trusted.

The system should grow only as fast as its test strategy can keep it understandable, diagnosable, and safe.
