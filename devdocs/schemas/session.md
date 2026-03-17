# Session JSONL schema

This document defines every `SessionLine` variant written to session files, the stale-file detection algorithm, and a canonical 3-turn example.

---

## Schema version

The first line of every session file is a metadata line that includes `schema_version` (integer). Current version: `1`. Absence is treated as version `0` (legacy). When reading, if `schema_version` is greater than the version the binary supports, loading fails with a clear error. Migrations are applied when `schema_version` is less than current (see development-plan Phase 9.7.1).

---

## SessionLine variants

Each line is a single JSON object with a `type` (or equivalent) field to discriminate the variant.

### Meta (session start)

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"meta"` |
| `session_id` | string | UUID v4 |
| `schema_version` | integer | Session schema version (e.g. 1) |
| `start_time` | string | ISO 8601 UTC (e.g. `"2026-03-16T12:00:00Z"`) |
| `project_path` | string | Absolute path to project (cwd at session start) |

### UserMessage

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"user_message"` |
| `role` | string | `"user"` |
| `content` | array | Array of content block objects (same shape as Message content) |

### AssistantMessage

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"assistant_message"` |
| `content` | array | Array of ContentBlock objects (text, tool_use) |

### ToolCall

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"tool_call"` |
| `tool_use_id` | string | Id of the tool call |
| `tool_name` | string | e.g. `"Read"`, `"Edit"` |
| `input` | object | Tool input as JSON |

### ToolResult

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"tool_result"` |
| `tool_use_id` | string | Matches the corresponding ToolCall |
| `content` | string | Result text |
| `is_error` | boolean | True if the tool failed |
| `duration_ms` | integer | Optional; execution time in milliseconds |

For **Edit** and **Write**, also record (for stale-file detection):

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Absolute path of the file edited/written |
| `content_hash` | string | SHA-256 of file content after the edit (hex) |
| `mtime_nanos` | integer | File mtime in nanoseconds since epoch after the edit |

These fields are used on resume to detect if the file was changed externally (see Stale-file detection below).

### System

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"system"` |
| `subtype` | string | e.g. `"compact_boundary"`, `"mode_change"` |
| `message` | string | Optional human-readable note |

### Result (session end or turn end)

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"result"` |
| `exit_status` | string | `"completed"` \| `"max_turns"` \| `"budget_exceeded"` \| `"error"` |
| `total_cost_usd` | float | Cumulative session cost |
| `num_turns` | integer | Number of turns |
| `duration_ms` | integer | Total duration |

---

## Stale-file detection algorithm

**When recording:** On every successful **Edit** or **Write** tool execution, append to the session line (or to a side structure keyed by turn) the triple `(path, content_hash, mtime_nanos)` for the file that was modified. Store these in the ToolResult line for that tool call (fields `path`, `content_hash`, `mtime_nanos`).

**On resume (`--resume <session_id>`):** Before reconstructing the loop:

1. Load the session JSONL and collect all ToolResult lines that have `path`, `content_hash`, and `mtime_nanos`.
2. For each such path, read the current file: compute its content hash and mtime.
3. If for any path the current content hash or mtime differs from the stored value, the file is considered **stale**.
4. If any file is stale:
   - **Interactive mode:** Warn the user: "The following files were modified since this session: … Continue anyway? [y/N]". If no, abort resume. If yes, continue (optionally with `--resume-ignore-stale` semantics).
   - **Non-interactive mode:** Exit with error unless `--resume-ignore-stale` was passed. Error message: "Cannot resume: file(s) modified since session: … Use --resume-ignore-stale to continue anyway."

This ensures that resume does not apply further edits on top of unexpectedly changed files without the user acknowledging it.

---

## Canonical 3-turn example (raw JSONL)

```jsonl
{"type":"meta","session_id":"a1b2c3d4-e5f6-7890-abcd-ef1234567890","schema_version":1,"start_time":"2026-03-16T12:00:00Z","project_path":"/tmp/repo"}
{"type":"user_message","role":"user","content":[{"type":"text","text":"What's in src/main.rs?"}]}
{"type":"assistant_message","content":[{"type":"tool_use","id":"toolu_01","name":"Read","input":{"path":"src/main.rs"}}]}
{"type":"tool_call","tool_use_id":"toolu_01","tool_name":"Read","input":{"path":"src/main.rs"}}
{"type":"tool_result","tool_use_id":"toolu_01","content":"fn main() {\n    println!(\"Hello\");\n}\n","is_error":false,"duration_ms":2}
{"type":"assistant_message","content":[{"type":"text","text":"The file contains a simple main that prints Hello."}]}
{"type":"result","exit_status":"completed","total_cost_usd":0.002,"num_turns":1,"duration_ms":1500}
```

This represents: one user message, one assistant turn that issues Read, one tool result, one assistant text reply, and a final result line.
