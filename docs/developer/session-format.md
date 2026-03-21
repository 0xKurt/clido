# Session Format

Sessions are stored as newline-delimited JSON (JSONL) files. Each line is a self-contained JSON object with a `type` discriminant field.

## File location

```
{data_dir}/sessions/{project_hash}/{session_id}.jsonl
```

| Platform | `data_dir` |
|----------|-----------|
| Linux | `~/.local/share/clido` |
| macOS | `~/Library/Application Support/clido` |

`project_hash` is a lowercase hex string derived from the absolute project path (SHA-256 prefix). `session_id` is a UUID4 without hyphens.

Example:

```
~/.local/share/clido/sessions/a3f8b2c1d4e5678f/a1b2c3d4e5f6789abcdef0123456789a.jsonl
```

## Schema version

The current schema version is `1` (constant `SCHEMA_VERSION` in `clido-storage`).

## Line types

Every line in the file has a `"type"` field. Lines appear in chronological order.

### `meta` — First line of every session

```json
{
  "type": "meta",
  "session_id": "a1b2c3d4e5f6789abcdef0123456789a",
  "schema_version": 1,
  "start_time": "2026-03-21T14:30:00Z",
  "project_path": "/home/user/projects/my-app"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | UUID4 (no hyphens) |
| `schema_version` | integer | Always `1` for current sessions |
| `start_time` | string | ISO 8601 UTC |
| `project_path` | string | Absolute path of the working directory |

### `user_message` — A message from the user

```json
{
  "type": "user_message",
  "role": "user",
  "content": [
    {
      "type": "text",
      "text": "Refactor the parse() function to return Result<T, ParseError>"
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `role` | string | Always `"user"` |
| `content` | array | Content blocks (see below) |

### `assistant_message` — A response from the LLM

```json
{
  "type": "assistant_message",
  "content": [
    {
      "type": "text",
      "text": "I'll refactor the parse() function now. Let me start by reading it."
    },
    {
      "type": "tool_use",
      "id": "toolu_01abc123",
      "name": "Read",
      "input": { "file_path": "src/parser.rs" }
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `content` | array | Mix of `text` and `tool_use` content blocks |

### `tool_call` — A tool call extracted from an assistant message

```json
{
  "type": "tool_call",
  "tool_use_id": "toolu_01abc123",
  "tool_name": "Read",
  "input": { "file_path": "src/parser.rs" }
}
```

::: tip
`tool_call` lines are written for each tool use in an assistant message. They are a convenience index — the same information is in the `assistant_message` content. Resume logic uses the `assistant_message` directly.
:::

### `tool_result` — The result of a tool call

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01abc123",
  "content": "fn parse(input: &str) -> Option<Ast> {\n    // ...\n}",
  "is_error": false,
  "duration_ms": 8,
  "path": "src/parser.rs",
  "content_hash": "sha256:a1b2c3d4...",
  "mtime_nanos": 1742560200000000000
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tool_use_id` | string | Yes | Matches the `tool_use.id` in the assistant message |
| `content` | string | Yes | Tool output text |
| `is_error` | boolean | Yes | Whether the tool returned an error |
| `duration_ms` | integer | No | Tool execution time |
| `path` | string | No | File path (for Read/Write/Edit tools; used for stale detection) |
| `content_hash` | string | No | `sha256:<hex>` of the file at read time |
| `mtime_nanos` | integer | No | File modification time at read time |

`path`, `content_hash`, and `mtime_nanos` are set by file-reading tools (Read, Glob, Grep) for stale file detection on resume.

### `system` — System-generated events

```json
{
  "type": "system",
  "subtype": "compaction",
  "message": "Context compacted: 42 turns → 1 summary (saved ~18,000 tokens)"
}
```

```json
{
  "type": "system",
  "subtype": "error",
  "message": "Provider error: rate limit exceeded, retrying in 5s"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `subtype` | string | `"compaction"`, `"error"`, `"warning"`, `"info"` |
| `message` | string (optional) | Human-readable description |

### `result` — Last line of a completed session

```json
{
  "type": "result",
  "exit_status": "success",
  "total_cost_usd": 0.0034,
  "num_turns": 4,
  "duration_ms": 8312
}
```

| Field | Type | Description |
|-------|------|-------------|
| `exit_status` | string | `"success"`, `"max_turns"`, `"max_budget"`, `"error"`, `"interrupted"` |
| `total_cost_usd` | float | Accumulated cost for the session |
| `num_turns` | integer | Number of completed turns |
| `duration_ms` | integer | Total wall time from start to end |

## Content block types

Content blocks appear inside `user_message.content` and `assistant_message.content` arrays.

### `text`

```json
{ "type": "text", "text": "The function has 42 lines." }
```

### `tool_use` (assistant messages only)

```json
{
  "type": "tool_use",
  "id": "toolu_01abc123",
  "name": "Bash",
  "input": { "command": "cargo check" }
}
```

### `tool_result` (user messages only — for feeding results back to the LLM)

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01abc123",
  "content": "   Compiling my-app v0.1.0\n    Finished dev target in 1.2s",
  "is_error": false
}
```

## A complete session example

```json
{"type":"meta","session_id":"a1b2c3d4e5f6789abcdef0123456789a","schema_version":1,"start_time":"2026-03-21T14:30:00Z","project_path":"/home/user/projects/my-app"}
{"type":"user_message","role":"user","content":[{"type":"text","text":"How many lines is src/main.rs?"}]}
{"type":"assistant_message","content":[{"type":"tool_use","id":"toolu_01abc","name":"Read","input":{"file_path":"src/main.rs"}}]}
{"type":"tool_call","tool_use_id":"toolu_01abc","tool_name":"Read","input":{"file_path":"src/main.rs"}}
{"type":"tool_result","tool_use_id":"toolu_01abc","content":"fn main() {\n...","is_error":false,"duration_ms":5,"path":"src/main.rs","content_hash":"sha256:deadbeef...","mtime_nanos":1742560200000000000}
{"type":"assistant_message","content":[{"type":"text","text":"src/main.rs has 312 lines."}]}
{"type":"result","exit_status":"success","total_cost_usd":0.0009,"num_turns":1,"duration_ms":2100}
```

## Reading sessions programmatically

```rust
use clido_storage::{SessionReader, SessionLine};

let lines: Vec<SessionLine> = SessionReader::open(&session_path)?
    .collect::<anyhow::Result<Vec<_>>>()?;

for line in &lines {
    match line {
        SessionLine::UserMessage { content, .. } => { /* ... */ }
        SessionLine::AssistantMessage { content } => { /* ... */ }
        SessionLine::ToolResult { tool_name, content, .. } => { /* ... */ }
        SessionLine::Result { total_cost_usd, .. } => { /* ... */ }
        _ => {}
    }
}
```

Or with shell tools:

```bash
# Extract all user messages
jq -r 'select(.type == "user_message") | .content[] | select(.type == "text") | .text' session.jsonl

# Total cost across all sessions
jq -r 'select(.type == "result") | .total_cost_usd' ~/.local/share/clido/sessions/*/*.jsonl | paste -sd+ | bc
```
