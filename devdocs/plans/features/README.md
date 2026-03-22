# Feature Plans

Detailed implementation plans for features that close the gap between clido and
Cursor / Claude Code / Cline — and in several cases, improve on all of them.

Each plan contains: problem statement, competitive analysis, our improvements,
full design, crate/file-level implementation steps, config schema, CLI surface,
TUI changes, slash commands, complete test plan, docs updates, and a Definition of Done.

## Plans

| File | Feature | Priority | Effort | Status |
|------|---------|----------|--------|--------|
| [project-rules.md](project-rules.md) | Project Rules File (`CLIDO.md`) | P0 | Low | ✅ Done |
| [gitignore-index.md](gitignore-index.md) | `.gitignore`-aware Repository Index | P0 | Low | ✅ Done |
| [edit-reliability.md](edit-reliability.md) | Edit Tool Multi-Strategy Patching | P0 | Medium | ✅ Done |
| [diff-preview.md](diff-preview.md) | Diff Preview and Approval Before Write | P1 | Medium | ✅ Done |
| [plan-mode.md](plan-mode.md) | Interactive Plan Mode with TUI Editor | P1 | High | ✅ Done |
| [checkpoint-rollback.md](checkpoint-rollback.md) | Checkpoint and Rollback | P1 | Medium | ✅ Done |
| [web-tools.md](web-tools.md) | Web Fetch and Search Tools | P1 | Medium | ✅ Done |
| [git-awareness.md](git-awareness.md) | Native Git Awareness | P1 | Medium | ✅ Done |
| [notifications.md](notifications.md) | Desktop Notifications and Completion Hooks | P2 | Low | ✅ Done |
| [auto-test-loop.md](auto-test-loop.md) | Automatic Test Loop | P2 | High | ✅ Done |
| [lsp-diagnostics.md](lsp-diagnostics.md) | LSP / Compiler Diagnostics Tool | P2 | Medium | ✅ Done |
| [model-switching.md](model-switching.md) | Mid-Session Model Switching | P2 | Medium | ✅ Done |
| [image-input.md](image-input.md) | Image and Screenshot Input | P3 | Medium | ✅ Done |

## Priority rationale

**P0 — Fix first** (bugs / severe UX gaps):
- `project-rules.md` — power users expect this; it's a 1-file read + inject
- `gitignore-index.md` — the index is currently unusable on any real project (indexes `node_modules`, `target/`)
- `edit-reliability.md` — silent failures on exact-match are the #1 agent reliability issue

**P1 — High value, core differentiators:**
- `diff-preview.md` — safety before write, industry standard
- `plan-mode.md` — our best differentiator; full editable DAG vs competitors' text blobs
- `checkpoint-rollback.md` — safety net; P0 for users without git
- `web-tools.md` — lookup tasks are ~30% of real sessions
- `git-awareness.md` — essential for commit/PR workflows

**P2 — Quality of life:**
- `notifications.md` — trivial to implement, high flow improvement
- `auto-test-loop.md` — very common pattern; complex to do well
- `lsp-diagnostics.md` — structured errors vs. parsing Bash output
- `model-switching.md` — cost optimization + quality routing

**P3 — Expansion:**
- `image-input.md` — niche but impactful for UI/design work

## Implementation order suggestion

```
Phase A (1–2 days):  gitignore-index, project-rules, notifications
Phase B (3–5 days):  edit-reliability, diff-preview, checkpoint-rollback
Phase C (1 week):    plan-mode (TUI editor), web-tools, git-awareness
Phase D (1 week):    auto-test-loop, lsp-diagnostics, model-switching
Phase E (future):    image-input
```
