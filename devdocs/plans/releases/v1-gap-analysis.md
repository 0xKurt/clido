# V1 Gap Analysis

This document compares the current implementation (as of the last update) against the V1 plan ([v1.md](v1.md)), [cli-interface-specification.md](../cli-interface-specification.md), [development-plan.md](../development-plan.md), and [config.md](../../schemas/config.md). **Done** = implemented; **Gap** = missing or incomplete.

---

## Build and tests

| Check | Status |
|-------|--------|
| `cargo build --all-targets` | Done |
| `cargo test` (all crates) | Done |
| Doctests disabled where needed (mixed toolchain) | Done |

---

## Phase 3 — Foundation (MVA)

### 3.1 Tools

| Item | Status | Notes |
|------|--------|--------|
| ReadTool (file_path/path, offset, limit, line prefix `N→`) | Done | file_path + path alias |
| WriteTool (path, content, create_dir_all) | Done | Uses resolve_for_write |
| EditTool (old_string, new_string, replace_all, exact error format) | Done | |
| GlobTool (pattern, path, ignore walk, sort by mtime) | Done | Uses glob::Pattern + ignore |
| GrepTool (pattern, path, output_mode, context, -i, head_limit, unknown param error) | Done | Subset of params; regex not grep crate |
| BashTool | Done | |
| All six registered with workspace_root | Done | default_registry() |
| Path guard: resolve_and_check, resolve_for_write, .. escape denied | Done | |

**Gaps:** Read tool schema uses both `file_path` and `path` (spec prefers file_path; alias is fine). Grep: dev plan lists -A/-B/-C, glob, type, multiline — V1 minimal set is implemented.

### 3.2 CLI (clap)

| Item | Status | Notes |
|------|--------|--------|
| clap with all V1 flags (profile, model, provider, max_turns, max_budget_usd, etc.) | Done | |
| `--version` / `clido version` | Done | version in Parser + version subcommand |
| sessions list, sessions show, legacy list-sessions, show-session with deprecation | Done | |
| Exit codes: 0 success, 1 runtime, 2 usage/config, 3 soft limit | Done | CliError::Usage(2), Runtime(1), SoftLimit(3) |
| Unsupported provider → helpful error at startup (not panic) | Done | Check before creating provider; exit 2 |
| API key error message (mention clido doctor) | Done | Spec message + "Run: clido doctor" |
| REPL when no prompt + TTY | Gap | Stub only; stdin read when non-TTY |
| init subcommand | Done | Creates config dir and default config.toml |

### 3.3 Session storage

| Item | Status | Notes |
|------|--------|--------|
| SessionLine variants (meta, user_message, assistant_message, tool_call, tool_result, system, result) | Done | |
| SessionWriter (create, write_line), SessionReader (load, stale_file_records) | Done | |
| list_sessions(project_path), SessionSummary | Done | |
| Paths: data_dir, session_dir_for_project, session_file_path | Done | |
| Per-turn session writing (user_message, assistant_message, tool_call, tool_result) | Done | Agent loop writes each turn; Write/Edit include path, content_hash, mtime_nanos |
| --resume: load history, reconstruct loop | Done | main loads session, reconstructs history, run_continue |
| Stale-file detection on resume | Done | stale_paths() used; prompt or error unless --resume-ignore-stale |
| --resume-ignore-stale | Done | Flag present |

### 3.4 Config loading

| Item | Status | Notes |
|------|--------|--------|
| Global config ~/.config/clido/config.toml, CLIDO_CONFIG | Done | config_loader.rs; CLIDO_CONFIG overrides path |
| Project config .clido/config.toml (walk upward) | Done | find_project_config; merge order global → project |
| Named profiles, [agent], [tools], [context] | Done | ConfigFile + ContextSection; agent_config_from_loaded |
| Profile selection (--profile), provider/model from config | Done | main resolves profile, CLI overrides |
| Unsupported provider error message | Done | validate_provider at startup |
| API key from profile api_key_env | Done | main reads env from profile |
| CLIDO.md / CLAUDE.md trust-on-first-use | Gap | Optional V1 |
| Tool guidance in system prompt | Gap | Phase 3.4.3 |

### 3.5 Streaming

| Item | Status | Notes |
|------|--------|--------|
| Anthropic streaming (complete_stream) | Gap | Provider returns empty stream |
| Rich streaming output (tool lifecycle pending → completed) | Gap | Not implemented |
| output_format text | Done | Default |
| output_format json / stream-json | Partial | json implemented; stream-json returns usage error |

### 3.6 Pricing

| Item | Status | Notes |
|------|--------|--------|
| pricing.toml in config reference | Done | config.md |
| Load pricing.toml from config dir | Done | load_pricing(); 90-day staleness warning on load |
| 90-day staleness warning | Done | pricing.rs + doctor |

---

## Phase 4.2, 4.3, 4.5 — Quality features

| Item | Status |
|------|--------|
| Context engine + compaction | Done | clido-context; assemble() with truncation fallback; integrated in agent loop |
| Permission system (Default, AcceptAll, PlanOnly) | Done | permission_mode in config/CLI; PlanOnly gate; AcceptAll executes without prompt |
| Plan mode, ExitPlanMode tool | Done | ExitPlanModeTool; permission_mode_override in loop |
| Serialized AskUser (one prompt at a time) | Done | AskUser trait; StdinAskUser in CLI when Default + TTY |

---

## Phase 5 — Hardening

| Item | Status |
|------|--------|
| Robust error handling and retries | Done | Anthropic: 429/5xx retry with backoff; ClidoError::Interrupted |
| Session recovery (--resume wire) | Done |
| Graceful shutdown (SIGINT/SIGTERM) | Done | cancel flag; exit 130; session flush on interrupt |
| Integration test suite, V1 critical-path tests | Done | Unit tests tools/storage/config; integration tests (help, doctor, init); nextest in CI |
| Edit safety (stale-file on resume, dirty list) | Done | stale_paths(); --resume-ignore-stale |

---

## Phase 8.4 — clido doctor

| Item | Status |
|------|--------|
| doctor subcommand | Done |
| Check: API key | Done |
| Check: session dir writable | Done |
| Check: pricing.toml presence and staleness | Done |
| Exit codes 0 / 1 / 2 per spec | Done |

---

## Exit criteria (v1.md) — summary

| Criterion | Status |
|-----------|--------|
| Six core tools complete multi-step tasks | Done (tools exist; unit + integration tests) |
| Context compaction | Done |
| Plan mode / permission-gated tools | Done |
| Resume with stale-file detection | Done |
| Coverage ≥ 70%, critical-path tests | Unit + integration tests; nextest in CI (coverage target optional) |
| Unsupported provider → helpful error | Done |
| clido doctor V1 checks | Done |
| pricing.toml present + 90-day warning | Done |

---

## Applied fixes (this pass)

1. **Exit codes:** `CliError` enum with `Usage` (2), `Runtime` (1), `SoftLimit` (3), `Interrupted` (130). Missing prompt, API key, unsupported provider → 2; agent max_turns/budget error → 3. Exit code 130 for SIGINT is documented in CLI spec.
2. **--version:** `#[command(version = env!("CARGO_PKG_VERSION"))]` on Cli; `clido version` subcommand unchanged.
3. **Unsupported provider:** If `--provider` is set and not `anthropic`, return `CliError::Usage` with message and exit 2.
4. **API key error:** Message per spec including "Run: clido doctor to check all configuration."

## Code-level bugs (fixed)

- **Resume/continue:** `--resume` and `--continue` wired in main; session load, stale-file check, history reconstruction, `run_continue()`.
- **Permission mode and allowlists:** `permission_mode` and allowed/disallowed tools applied; PlanOnly gate in agent loop; filtered registry.
- **System prompt:** `--system-prompt-file` and `--append-system-prompt` applied when building config.
- **Output format / verbose:** `--output-format json` prints result JSON; `stream-json` returns usage error; `-v` sets log level to debug.
- **Budget and soft limits:** `ClidoError::BudgetExceeded` and `MaxTurnsExceeded`; agent loop enforces `max_budget_usd`; main maps to exit 3.
- **Per-turn session:** Agent loop writes UserMessage, AssistantMessage, ToolCall, ToolResult each turn; Write/Edit ToolResult include path, content_hash, mtime_nanos.

---

## References

- [v1.md](v1.md) — V1 plan and exit criteria
- [cli-interface-specification.md](../cli-interface-specification.md) — CLI surface, exit codes, errors
- [development-plan.md](../development-plan.md) — Phase 3–5 milestones
- [config.md](../../schemas/config.md) — Config and pricing reference
- [session.md](../../schemas/session.md) — Session JSONL schema
