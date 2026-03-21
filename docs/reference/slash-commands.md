# Slash Commands (TUI)

Slash commands are typed in the TUI input field and executed immediately when you press Enter. They are only available in the interactive TUI — not in CLI / non-TTY mode.

## Command list

| Command | Description | Example | Notes |
|---------|-------------|---------|-------|
| `/help` | Display all available slash commands | `/help` | Output appears in the chat pane |
| `/sessions` | Open the session picker | `/sessions` | Use arrow keys to select, Enter to open |
| `/new` | Start a new session, discarding the current one | `/new` | The current session file is preserved; you just stop using it |
| `/clear` | Clear the chat display | `/clear` | The session JSONL file is not modified; history is preserved |
| `/plan` | Show the current planner task graph | `/plan` | Only meaningful when `--planner` is active |
| `/memory <query>` | Search long-term memory and display matches | `/memory error handling` | Full-text search over memory DB |
| `/cost` | Print accumulated cost and token usage for this session | `/cost` | Equivalent to the status strip numbers |
| `/model` | Show the currently active model and provider | `/model` | |
| `/tools` | List all available tools (built-in + MCP) | `/tools` | |
| `/quit` | Exit clido | `/quit` | Equivalent to pressing `Ctrl+C` when idle |

## Using slash commands

Type a `/` followed by the command name in the input field:

```
> /sessions
```

Press Enter to execute. Commands that produce output render it as a system message in the chat pane (visually distinct from user and assistant messages).

### Commands with arguments

`/memory` accepts a search query as the rest of the line:

```
> /memory refactor authentication module
```

```
[memory search: "refactor authentication module"]
  • User prefers JWT over session cookies (2026-03-15)
  • Auth module was refactored to use tower-service (2026-03-10)
  • AuthError variants: Expired, Invalid, MissingToken (2026-03-08)
```

## Session picker

`/sessions` opens a full-screen picker overlay:

```
╭─ Sessions ──────────────────────────────────────────────────────────────────╮
│  Filter: _                                                                    │
│                                                                               │
│  > a1b2c3  2026-03-21  "Refactor the parser module"   ~/projects/app  $0.02  │
│    d4e5f6  2026-03-20  "Add unit tests for lexer"      ~/projects/app  $0.04  │
│    789abc  2026-03-19  "Fix memory leak in pool"       ~/projects/lib  $0.02  │
╰─────────────────────────────────────────────────────────────────────────────╯
  ↑/↓ navigate  Enter open  Escape cancel  Type to filter
```

| Key | Action |
|-----|--------|
| `Up` / `Down` | Move selection |
| `Enter` | Open the selected session |
| `Escape` | Close the picker without changing sessions |
| Any text | Filter sessions by ID prefix or preview text |

## Difference from CLI commands

TUI slash commands are distinct from CLI subcommands. For example, `/sessions` in the TUI opens the picker, while `clido sessions list` on the command line prints a table. See [CLI Reference](/reference/cli) for the full list of CLI commands.
