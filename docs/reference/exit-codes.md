# Exit Codes

clido uses the following exit codes:

| Code | Name | When |
|------|------|------|
| `0` | Success | The agent completed the task successfully |
| `1` | Error | A runtime error occurred (provider error, tool failure, unexpected panic) |
| `2` | Config / usage error | Bad flag, unknown provider, missing required config, invalid config file |
| `3` | Soft limit | The agent was stopped due to `--max-turns` or `--max-budget-usd` being reached |
| `130` | Interrupted | The user pressed Ctrl+C |

## Code 0 — Success

The agent ran to completion and returned a response. The task may or may not have been completed to the user's satisfaction — this is the LLM's judgment, not clido's.

```bash
clido "list files"
echo $?  # 0
```

## Code 1 — Error

A non-recoverable error occurred. Examples:

- The provider API returned a persistent error (rate limit exhausted, authentication failure, server error)
- A required file could not be read
- An unexpected internal error (bug in clido)

Error details are printed to stderr.

```bash
ANTHROPIC_API_KEY=invalid clido "test"
echo $?  # 1
```

## Code 2 — Config / Usage error

A configuration or command-line usage problem. Examples:

- Unknown flag passed
- `--provider` set to an unrecognised value
- Referenced profile does not exist in `config.toml`
- Config file is malformed TOML
- Required input not provided to a workflow

```bash
clido --unknown-flag "test"
echo $?  # 2
```

## Code 3 — Soft limit

The agent was stopped because a resource limit was reached. The agent's partial output (if any) is still printed. This is not an error — it is an expected outcome when the task is larger than the configured limits.

```bash
clido --max-turns 1 "do a huge task"
echo $?  # 3
```

In JSON output mode, the `exit_status` field will be `"max_turns"` or `"max_budget"`.

## Code 130 — Interrupted

The user pressed Ctrl+C (SIGINT). The agent loop is cancelled and clido exits immediately.

```bash
clido "long running task"
# press Ctrl+C
echo $?  # 130
```

## Checking exit codes in scripts

```bash
#!/usr/bin/env bash
set -e

clido --max-budget-usd 0.50 --output-format json "refactor module" > result.json
EXIT=$?

case $EXIT in
  0) echo "Success" ;;
  1) echo "Error — check stderr" >&2; exit 1 ;;
  2) echo "Config error" >&2; exit 2 ;;
  3) echo "Budget or turn limit reached — partial result in result.json" ;;
  130) echo "Interrupted" >&2; exit 130 ;;
esac
```
