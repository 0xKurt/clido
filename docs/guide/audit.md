# Audit Log

clido logs every tool call to an append-only audit log. The audit log lets you review exactly what the agent did, when, how long each tool took, and whether it succeeded.

## What is logged

For every tool call, the audit log records:

| Field | Description |
|-------|-------------|
| `timestamp` | ISO 8601 UTC timestamp |
| `session_id` | Session ID this call belongs to |
| `tool_name` | Name of the tool (e.g. `Bash`, `Read`, `Edit`) |
| `input` | Tool input as JSON |
| `output_snippet` | First 500 characters of the tool output |
| `is_error` | Whether the tool returned an error |
| `duration_ms` | Tool execution time in milliseconds |

## Viewing the audit log

```bash
clido audit
```

Prints the full audit log in table form, most recent last:

```
TIMESTAMP             SESSION   TOOL   DURATION  STATUS  INPUT
2026-03-21T14:32:01Z  a1b2c3    Bash   1243ms    ok      {"command":"cargo check"}
2026-03-21T14:32:04Z  a1b2c3    Read   8ms       ok      {"file_path":"src/main.rs"}
2026-03-21T14:32:05Z  a1b2c3    Edit   3ms       ok      {"file_path":"src/main.rs",...}
2026-03-21T14:32:09Z  a1b2c3    Bash   3821ms    ok      {"command":"cargo test"}
```

## Filtering

### Tail mode

Print only the last N entries:

```bash
clido audit --tail 20
```

### Filter by session

Show only entries from a specific session (prefix match):

```bash
clido audit --session a1b2c3
```

### Filter by tool

Show only calls to a specific tool:

```bash
clido audit --tool Bash
clido audit --tool Edit
```

### Filter by date

Show only entries since a specific date (ISO 8601 date or datetime):

```bash
clido audit --since 2026-03-01
clido audit --since 2026-03-21T12:00:00Z
```

### Combining filters

Filters can be combined:

```bash
clido audit --tail 100 --tool Bash --since 2026-03-20
```

## JSON output

Output the audit log as newline-delimited JSON for scripting:

```bash
clido audit --json
```

```json
{"timestamp":"2026-03-21T14:32:01Z","session_id":"a1b2c3...","tool_name":"Bash","input":{"command":"cargo check"},"is_error":false,"duration_ms":1243}
{"timestamp":"2026-03-21T14:32:04Z","session_id":"a1b2c3...","tool_name":"Read","input":{"file_path":"src/main.rs"},"is_error":false,"duration_ms":8}
```

Combine with `jq` for powerful queries:

```bash
# Average Bash execution time
clido audit --tool Bash --json | jq -s '[.[].duration_ms] | add/length'

# Count errors by tool
clido audit --json | jq -r 'select(.is_error) | .tool_name' | sort | uniq -c

# All commands run today
clido audit --tool Bash --since 2026-03-21 --json | jq -r '.input.command'
```

## Storage location

The audit log is stored per-project:

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/clido/audit/<project-hash>/audit.jsonl` |
| macOS | `~/Library/Application Support/clido/audit/<project-hash>/audit.jsonl` |

The project hash is derived from the absolute path of the working directory. This keeps audit logs for different projects separate.

Override the base data directory with `CLIDO_DATA_DIR`.

## Audit log format

The audit log is a plain JSONL file. Each line is an independent JSON object:

```json
{
  "timestamp": "2026-03-21T14:32:01Z",
  "session_id": "a1b2c3d4e5f6789abcdef0123456789",
  "tool_name": "Bash",
  "input": { "command": "cargo check" },
  "output_snippet": "   Compiling my-app v0.1.0\n    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.24s",
  "is_error": false,
  "duration_ms": 1243
}
```

The file can be read directly without the `clido audit` command:

```bash
tail -f ~/.local/share/clido/audit/*/audit.jsonl | jq .
```
