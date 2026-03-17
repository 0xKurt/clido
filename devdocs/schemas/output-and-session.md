# Output, session, and audit schemas

This document is the **single schema-focused reference** for:

- Session JSONL line types (with pointer to full definition)
- `--output-format json` result schema
- `--output-format stream-json` event types
- Audit log record format
- Schema migration and versioning policy

Session file format and stale-file detection are fully specified in [session.md](session.md).

---

## 1. Session JSONL (session files)

Session files are newline-delimited JSON. Each line is a **SessionLine** variant. The canonical list of variants, field-by-field, and the 3-turn example are in [session.md](session.md).

**Summary of line types:**

| `type` | Description |
|--------|-------------|
| `meta` | Session start: session_id, schema_version, start_time, project_path |
| `user_message` | User content blocks |
| `assistant_message` | Assistant content (text, tool_use) |
| `tool_call` | Tool invocation: tool_use_id, tool_name, input |
| `tool_result` | Tool outcome: content, is_error, duration_ms; for Edit/Write also path, content_hash, mtime_nanos |
| `system` | System events: subtype (e.g. compact_boundary, mode_change), message |
| `result` | Session/turn end: exit_status, total_cost_usd, num_turns, duration_ms |

**Schema version:** First line must include `schema_version` (integer). Current session schema version: `1`. See §5 for versioning policy.

---

## 2. `--output-format json` (final result)

Emitted as a **single JSON object** on stdout when the run completes. Warnings go to stderr. Used for scripting and CI.

**Schema:**

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | integer | Output schema version (currently 1) |
| `type` | string | `"result"` |
| `exit_status` | string | `completed` \| `max_turns_reached` \| `budget_exceeded` \| `error` |
| `result` | string | Final assistant text response |
| `session_id` | string | UUID of the session |
| `num_turns` | integer | Number of turns |
| `duration_ms` | integer | Total duration in milliseconds |
| `total_cost_usd` | float | Cumulative cost |
| `is_error` | boolean | True if exit_status is `error` or runtime failure |
| `usage` | object | Optional token usage (see below) |

**`usage` object (optional):**

| Field | Type |
|-------|------|
| `input_tokens` | integer |
| `cache_read_input_tokens` | integer |
| `output_tokens` | integer |

**Example:**

```json
{
  "schema_version": 1,
  "type": "result",
  "exit_status": "completed",
  "result": "Refactored error handling in src/main.rs.",
  "session_id": "a3f2b1c9-...",
  "num_turns": 5,
  "duration_ms": 12345,
  "total_cost_usd": 0.0045,
  "is_error": false,
  "usage": {
    "input_tokens": 1234,
    "cache_read_input_tokens": 890,
    "output_tokens": 567
  }
}
```

---

## 3. `--output-format stream-json` (streaming events)

Newline-delimited JSON: one JSON object per line on stdout. Each object has a `type` (and optionally `subtype`) to discriminate the event.

**Event types:**

| `type` | `subtype` (if any) | Fields (typical) | Description |
|--------|--------------------|-------------------|-------------|
| `text` | — | `text` | Streaming assistant text chunk |
| `tool_use` | — | `name`, `input` | Tool invocation (name e.g. Read, input object) |
| `tool_result` | — | `name`, `is_error`, `content` | Tool result |
| `system` | `compact_start` | `context_tokens` | Context compaction started |
| `system` | `compact_end` | `summary_tokens` | Context compaction finished |
| `result` | — | `exit_status`, `schema_version`, `session_id`, `num_turns`, `duration_ms`, `total_cost_usd`, … | Final result (same shape as §2) |

Progress events (e.g. tool pending/in_progress) are not emitted by default; use `--verbose` to include them if defined.

**Example stream (excerpt):**

```json
{"type":"text","text":"I'll read the file first.\n"}
{"type":"tool_use","name":"Read","input":{"path":"src/main.rs"}}
{"type":"tool_result","name":"Read","is_error":false,"content":"fn main() { ... }\n"}
{"type":"system","subtype":"compact_start","context_tokens":12847}
{"type":"system","subtype":"compact_end","summary_tokens":600}
{"type":"result","exit_status":"completed","schema_version":1,"session_id":"...","num_turns":3,"duration_ms":5000,"total_cost_usd":0.003}
```

---

## 4. Audit log (V2)

**File:** `{data_dir}/audit.jsonl`. Append-only; never compacted. Separate from session files.

**Record shape (one JSON object per line):**

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | string | ISO 8601 UTC |
| `session_id` | string | Session UUID |
| `tool_name` | string | e.g. `Read`, `Edit`, `mcp__fs__read_file` |
| `input_summary` | string | Short summary of input (no secrets; redacted if needed) |
| `result_summary` | string | Short summary of result (success/failure, size, etc.) |
| `is_error` | boolean | True if the tool call failed |
| `duration_ms` | integer | Execution time in milliseconds |

Secret redaction applies: tool inputs and results must not be written in full. See [security-model.md](../guides/security-model.md) §4.

**Availability:** V2 (Phase 7.3). Not present in V1.

---

## 5. Schema migration and versioning policy

- **Session files:** The first line (meta) includes `schema_version`. If the reader supports version N and the file has version M:
  - M > N: loading fails with a clear error (no silent ignore).
  - M < N: the loader may apply documented migrations (e.g. Phase 9.7.1) to upgrade to N in memory; the on-disk file is not rewritten unless the user explicitly exports/saves.
- **Output formats:** JSON and stream-json result objects include `schema_version`. Consumers should check it; unknown versions should be handled gracefully (e.g. fail closed or warn).
- **Backward compatibility:** New schema versions may add optional fields. Required fields are not removed. Renaming or removing a required field requires a new major schema version and must be documented in the changelog and this doc.
- **Audit log:** Audit lines do not currently have a schema_version field; the format is stable. If the audit format changes, a new file or a version field can be introduced and documented here.

Reference: [development-plan.md](../plans/development-plan.md) Phase 9.7.1 (session migration); [cli-interface-specification.md](../plans/cli-interface-specification.md) §10.
