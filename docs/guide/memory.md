# Memory

clido includes a long-term memory system that persists facts across sessions. The agent can store important observations during a session and retrieve them automatically in future sessions.

## What memory is

Memory is stored in a SQLite database with full-text search (FTS5). Each memory record is a short text snippet — a fact, preference, or observation — associated with a timestamp and optional tags.

The memory database lives at:

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/clido/memory.db` |
| macOS | `~/Library/Application Support/clido/memory.db` |

Override with `CLIDO_DATA_DIR`.

## How memory is injected

At the start of each agent turn, clido searches the memory database for records relevant to the current conversation. Matching memories are injected into the system prompt, providing the agent with context from previous sessions.

This happens automatically — you do not need to configure anything.

Example system prompt injection:

```
[Long-term memory]
- User prefers tabs over spaces in Rust code.
- This project uses the anyhow crate for error handling.
- The parse() function was refactored on 2026-03-15 to use Result<T, ParseError>.
```

The agent uses this context when making decisions. For example, if it knows you prefer tabs, it will not insert spaces when editing your code.

## Listing memories

```bash
clido memory list
```

```
ID    DATE        CONTENT
 1    2026-03-21  User prefers tabs over spaces in Rust code.
 2    2026-03-20  This project uses the anyhow crate for error handling.
 3    2026-03-19  The parse() function was refactored to use Result<T, ParseError>.
```

Limit the number of results:

```bash
clido memory list --limit 50
```

The default limit is 20.

## Searching memory from the TUI

In the TUI, use `/memory` followed by a search query:

```
/memory error handling
```

The agent searches the memory database and displays matching records in the chat pane.

## Pruning old memories

Keep only the N most recent memories, deleting older ones:

```bash
clido memory prune --keep 100
```

The default is to keep 100 memories if `--keep` is not specified.

## Resetting all memories

Delete all memories permanently:

```bash
clido memory reset
```

You will be asked for confirmation. Skip confirmation with `--force`:

```bash
clido memory reset --force
```

::: danger Irreversible
`clido memory reset` deletes all memories permanently. There is no undo.
:::

## How the agent creates memories

The agent decides autonomously when something is worth remembering. It will typically store:

- User preferences (code style, naming conventions)
- Project-specific facts (architecture decisions, recurring patterns)
- Information that is likely to be useful in future sessions

The agent does not store sensitive information (API keys, passwords) as memories — these are kept in the session JSONL only.

## Memory and privacy

Memories are stored locally on your machine. They are never sent to any external service. Only the relevant snippets (those matching the current context) are sent to the LLM provider as part of the system prompt.

## Disabling memory injection

Memory injection is enabled by default. To disable it for a session, set the system prompt explicitly, which replaces the default system prompt (including memory injection):

```bash
clido --system-prompt "You are a helpful assistant." "do something"
```

Or use a custom profile with a fixed system prompt in the config.
