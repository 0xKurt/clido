# Repository Index

The repository index is an optional file and symbol index that enables the `SemanticSearch` tool. When the index is built, the agent can quickly find relevant files and code symbols without reading the entire codebase.

## What the index is

The index is a SQLite database (`.clido/index.db`) that stores:

- **File records** — path, size, modification time, and language
- **Symbol records** — function names, struct names, type aliases, constants, etc., with their file and line number

When the agent calls the `SemanticSearch` tool, it queries this index using full-text search to find relevant code symbols and files. This is much faster than a grep over a large codebase and works for symbol-level queries like "find all implementations of the Serialize trait".

## Building the index

Index the current directory with default settings:

```bash
clido index build
```

Index a specific directory:

```bash
clido index build --dir /path/to/project
```

Index only specific file types (comma-separated extensions):

```bash
clido index build --ext rs,py,js,ts
```

Default extensions: `rs,py,js,ts,go`.

Building the index is idempotent — re-running it updates changed files and removes deleted ones.

::: tip Incremental updates
`clido index build` performs an incremental update: only files that have changed since the last build are re-indexed. For large codebases this is much faster than a full rebuild.
:::

## Checking index statistics

```bash
clido index stats
```

```
Index: .clido/index.db
Files indexed: 247
Symbols indexed: 3,891
Last updated: 2026-03-21 14:32:11 UTC
Size: 2.1 MB
```

## Clearing the index

Delete the index database entirely:

```bash
clido index clear
```

This removes `.clido/index.db`. Rebuild with `clido index build`.

## How the agent uses the index

When the index is present, clido automatically enables the `SemanticSearch` tool. The agent can call it with a query string:

```
[SemanticSearch] query: "parse error handling"
→ src/parser.rs:42  fn parse_with_error_context
→ src/errors.rs:10  struct ParseError
→ src/errors.rs:25  impl Display for ParseError
```

The agent uses these results to navigate to the right files rather than reading the entire codebase. This reduces token usage and speeds up responses for large projects.

## Supported languages

| Language | Extensions | Symbol types |
|----------|-----------|-------------|
| Rust | `.rs` | Functions, structs, enums, traits, type aliases, constants, modules |
| Python | `.py` | Functions, classes, methods |
| JavaScript | `.js`, `.jsx` | Functions, classes, arrow functions |
| TypeScript | `.ts`, `.tsx` | Functions, classes, interfaces, type aliases |
| Go | `.go` | Functions, types, methods, interfaces |

Additional languages can be added — see [Adding Tools](/developer/adding-tools) for extension points.

## Index storage location

The index is stored at `.clido/index.db` relative to the directory passed to `--dir` (or the current directory by default). This is a project-local file and can be added to `.gitignore`:

```
# .gitignore
.clido/index.db
```

## Enabling the index for the agent

The index is used automatically when `.clido/index.db` exists in the working directory. You can also explicitly enable the index via the config:

```toml
[agent]
# (no explicit index flag needed — presence of the DB enables it)
```

Or via the CLI:

```bash
# Build the index and immediately use it
clido index build && clido "find all functions that handle authentication"
```
