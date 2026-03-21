# All Flags

All top-level flags that can be passed to the root `clido` command or `clido run`.

Flags can also be set via environment variables — see the `Env` column.

| Flag | Short | Type | Default | Env | Description |
|------|-------|------|---------|-----|-------------|
| `--model` | — | string | profile model | `CLIDO_MODEL` | Override the model for this invocation |
| `--provider` | — | string | profile provider | `CLIDO_PROVIDER` | Override the provider for this invocation |
| `--profile` | — | string | `default` | `CLIDO_PROFILE` | Select a named profile from config.toml |
| `--workdir` | `-C` | path | `.` (current dir) | `CLIDO_WORKDIR` | Set the working directory |
| `--max-turns` | — | integer | `50` | `CLIDO_MAX_TURNS` | Maximum number of agent turns |
| `--max-budget-usd` | — | float | `5.0` | `CLIDO_MAX_BUDGET_USD` | Maximum spend per session in USD |
| `--permission-mode` | — | enum | `default` | `CLIDO_PERMISSION_MODE` | `default`, `accept-all`, or `plan` |
| `--output-format` | — | enum | `text` | `CLIDO_OUTPUT_FORMAT` | `text`, `json`, or `stream-json` |
| `--input-format` | — | enum | `text` | `CLIDO_INPUT_FORMAT` | `text` or `stream-json` |
| `--quiet` | `-q` | flag | false | — | Suppress spinner, tool output, cost footer |
| `--print` | `-p` | flag | false | — | Non-interactive mode (no REPL) |
| `--continue` | — | flag | false | — | Resume the newest session for the current dir |
| `--resume` | — | string | — | — | Resume a session by ID prefix |
| `--resume-ignore-stale` | — | flag | false | — | Skip stale file check when resuming |
| `--mcp-config` | — | path | — | — | Path to MCP server config (JSON) |
| `--sandbox` | — | flag | false | — | Enable Bash sandboxing |
| `--planner` | — | flag | false | — | Enable task decomposition planner (experimental) |
| `--max-parallel-tools` | — | integer | `4` | `CLIDO_MAX_PARALLEL_TOOLS` | Max concurrent read-only tool calls |
| `--system-prompt` | — | string | — | `CLIDO_SYSTEM_PROMPT` | Replace the default system prompt |
| `--system-prompt-file` | — | path | — | — | Read system prompt from a file |
| `--append-system-prompt` | — | string | — | — | Append text to the default system prompt |
| `--allowed-tools` | — | string | — | — | Comma-separated list of allowed tool names |
| `--disallowed-tools` | — | string | — | — | Comma-separated list of disallowed tool names |
| `--tools` | — | string | — | — | Alias for `--allowed-tools` |
| `--no-color` | — | flag | false | `NO_COLOR` | Disable ANSI color output |
| `--verbose` | `-v` | flag | false | — | Enable debug-level logging |

## Flag details

### `--model`

Override the model name for this invocation. Does not modify `config.toml`.

```bash
clido --model claude-3-haiku-20240307 "quick question"
```

### `--provider`

Override the provider for this invocation. Must be a valid provider name: `anthropic`, `openai`, `openrouter`, `local`.

```bash
clido --provider openrouter --model anthropic/claude-3-5-sonnet "task"
```

### `--profile`

Select a named profile from `config.toml`. Profiles define provider, model, and credentials.

```bash
clido --profile cheap "simple task"
```

### `--workdir` / `-C`

Set the working directory. All relative file paths and tool calls use this as the root.

```bash
clido -C /path/to/project "add tests"
```

### `--max-turns`

Maximum number of agent turns (one turn = one provider API call). When the limit is reached, the agent stops and clido exits with code 3.

Default: `50` (from config; CLI `--max-turns` default is `10` when explicitly set via flag to provide a sensible non-interactive cap).

### `--max-budget-usd`

Stop the agent when accumulated cost exceeds this value. Exits with code 3.

```bash
clido --max-budget-usd 0.25 "extensive refactor"
```

### `--permission-mode`

Controls how the agent handles state-changing tool calls:

| Value | Behaviour |
|-------|-----------|
| `default` | Prompt user in TUI; allow automatically in non-TTY mode |
| `accept-all` | Allow all tool calls without prompting |
| `plan` | Disallow all tool calls; agent can only plan and respond with text |

### `--output-format`

Controls the output format. See [Output Formats](/reference/output-formats).

### `--quiet` / `-q`

Suppress all output except the agent's final response text. No spinner, no tool lines, no cost footer.

### `--print` / `-p`

Non-interactive mode. Never enters the REPL. Useful in scripts where you want to guarantee clido does not wait for user input.

### `--continue`

Resume the most recent session for the current working directory. Equivalent to `--resume <latest-session-id>`.

### `--resume`

Resume a session by its ID (or a unique prefix). Combined with `--resume-ignore-stale` to skip the modified-file warning.

### `--mcp-config`

Path to a JSON file describing MCP servers to start. See [MCP Servers](/guide/mcp).

### `--sandbox`

Enable sandboxed Bash execution. Uses `sandbox-exec` on macOS and `bwrap` on Linux. The agent's shell commands run in a restricted environment that cannot access files outside the working directory or make network connections.

### `--planner`

Enable the experimental task decomposition planner. See [Planner](/guide/planner).

### `--max-parallel-tools`

Maximum number of read-only tool calls that can run concurrently. Increasing this can speed up tasks that do a lot of file reading.

### `--system-prompt`

Replace the default system prompt entirely. The default prompt includes project context and memory injection. Using `--system-prompt` disables these.

### `--append-system-prompt`

Append additional instructions to the default system prompt, preserving memory injection and project context.

### `--allowed-tools` / `--disallowed-tools`

Restrict which tools the agent can use. `--disallowed-tools` takes precedence over `--allowed-tools`.

```bash
# Only allow file reading tools
clido --allowed-tools "Read,Glob,Grep" "review this codebase"

# Prevent shell execution
clido --disallowed-tools Bash "refactor src/lib.rs"
```
