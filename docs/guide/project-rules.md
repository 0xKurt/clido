# Project Rules (CLIDO.md)

Clido supports a **project rules file** that injects custom instructions into the agent's system prompt automatically whenever you run clido in your project. This lets you encode team conventions, code style requirements, and project-specific guidance without repeating yourself in every prompt.

## What is CLIDO.md?

`CLIDO.md` is a Markdown file you place in your project directory. Its contents are prepended to the system prompt every time clido runs from within that project tree. Think of it as a persistent context file — the agent always knows your project conventions.

## Where to put it

Clido searches for rules files starting from the current working directory and walking up toward the filesystem root. It recognizes two locations per directory:

1. `.clido/rules.md` — hidden config directory, preferred for larger projects
2. `CLIDO.md` — visible root-level file, easy to commit and review

It also loads a **global rules file** at `~/.config/clido/rules.md` (lowest priority), which applies to all projects.

### Lookup hierarchy (priority order, highest last)

```
~/.config/clido/rules.md        ← global (lowest priority)
/parent-dir/CLIDO.md            ← ancestor directory
/parent-dir/.clido/rules.md
/your-project/CLIDO.md          ← project root
/your-project/.clido/rules.md   ← project hidden dir (highest priority)
```

Files closer to the current working directory take higher priority (their rules appear last in the assembled prompt). Within a single directory, `.clido/rules.md` is checked before `CLIDO.md`.

## Example CLIDO.md

```markdown
# My Project Rules

## Code style
- Use 4-space indentation.
- Prefer `const` over `let` where possible.
- All public functions must have doc comments.

## Testing
- Write unit tests for every non-trivial function.
- Test file names must match `*_test.rs` or be in a `tests/` module.

## Commit messages
- Use conventional commits: `feat:`, `fix:`, `chore:`, etc.
- Keep the subject line under 72 characters.
```

## Import directive

Rules files can import other Markdown files using the `[import: ./path/to/file.md]` directive on its own line:

```markdown
# Project Rules

[import: ./docs/style-guide.md]
[import: ./docs/testing-conventions.md]

## Additional notes
These are appended after the imported content.
```

Import paths are resolved relative to the file containing the directive. Imports are inlined at the position of the directive. Recursion is limited to 5 levels deep, and circular imports are detected and skipped automatically.

## CLI flags

### `--no-rules`

Skip all rules file discovery and injection for this invocation:

```sh
clido --no-rules "refactor this file"
```

Useful when you want a clean run without any project context.

### `--rules-file <path>`

Use a specific rules file instead of the standard hierarchical lookup:

```sh
clido --rules-file ./custom-rules.md "implement the feature"
```

You can also set this via the environment variable `CLIDO_RULES_FILE`.

## Config file options

You can set `no-rules` and `rules-file` persistently in your `.clido/config.toml`:

```toml
[agent]
no-rules = false
rules-file = "/path/to/my/rules.md"
```

## The `/rules` slash command

In the interactive TUI (REPL), type `/rules` to display a popup showing all currently active rules files and a preview of their content. Press **Enter** or **Esc** to dismiss the overlay.

This is useful for quickly verifying which rules files are being picked up in the current working directory.

## Token cost considerations

Rules files are injected into the system prompt on every turn, so large files will increase token usage. The `clido doctor` command warns you if any rules file exceeds 8,000 characters.

Keep your CLIDO.md concise — bullet points and short directives are more effective than lengthy prose.

## Checking rules with `clido doctor`

Run `clido doctor` to verify which rules files are found and their sizes:

```
✓ Rules files: /your-project/CLIDO.md (420 chars)
⚠ Rules file is large (10,240 chars) — may inflate token costs: /your-project/.clido/rules.md
```

If no rules files are found, doctor reports:

```
ℹ Rules files: none found (create CLIDO.md in project root)
```
