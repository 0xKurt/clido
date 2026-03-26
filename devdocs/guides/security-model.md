# Security model

This document is the single reference for Clido's security boundaries, path and secret handling, permission semantics, project-instruction trust, and sandbox behavior. It consolidates requirements that appear across the roadmap and test plan.

---

## 1. Workspace boundaries

- **Default root:** The agent's file tools (Read, Write, Edit, Glob, Grep) operate relative to the **current working directory** at session start. That directory is the workspace root for the run.
- **Configurable root:** An optional "root" directory can be enforced via config (e.g. a dedicated project path). Paths that escape this root are denied (see §2).
- **No implicit expansion:** The agent does not automatically read or write outside the workspace root. All file paths are validated before use.

---

## 2. Absolute path and path traversal policy

- **Canonicalization:** Before any file operation, the path is canonicalized (resolve `.`, `..`, and symlinks to their real path).
- **Root check:** If the canonical path does not lie under the configured workspace root (default: cwd), the tool returns `is_error: true` with content: `"Access denied: path outside working directory."`
- **Absolute paths:** A request for an absolute path (e.g. `/etc/passwd`) is allowed only if that path is under the workspace root; otherwise it is denied.
- **Trusted paths (optional):** Config may allow additional trusted paths outside the workspace for specific use cases; the default is cwd-only.

Implementation: [development-plan.md](../plans/development-plan.md) Phase 7.1.4 (Path traversal prevention for file tools).

---

## 3. Symlink escape policy

- **Resolution:** Symlinks are resolved during canonicalization. The security check is applied to the **resolved** path, not the link itself.
- **Escape via symlink:** If a symlink inside the workspace points to a location outside the workspace, following it yields a path outside the root; the operation is denied.
- **No special allowlist for symlinks:** There is no separate "allow symlink" flag; the only rule is that the resolved path must be under the workspace root.

Tests: [testing-strategy-and-master-test-plan.md](testing-strategy-and-master-test-plan.md) — path traversal attempts, symlink escapes.

---

## 4. Secret redaction rules

- **Logs and stdout/stderr:** Any line that would display an API key, token, or credential is redacted. Example: `ANTHROPIC_API_KEY=sk-ant-***...***` (only prefix/suffix or a fixed placeholder shown).
- **Session and audit logs:** Tool inputs and results that contain secret-like content must not be written in full to session JSONL or audit log. Either redact the value or store a placeholder (e.g. `"<redacted>"`).
- **Error messages:** Provider or config errors that might include tokens in the response body must be redacted before showing to the user.
- **CLI spec:** [cli-interface-specification.md](../plans/cli-interface-specification.md) §6 (Secret redaction).

---

## 5. Secret detection (Write/Edit content)

- **When:** Before writing content to disk (Write or Edit), optionally scan the content for known secret patterns (V1.5+). Gated by config: `[security] scan_writes = true`.
- **Patterns (examples):** `sk-[A-Za-z0-9]{48}`, `ghp_[A-Za-z0-9]{36}`, `-----BEGIN (RSA |EC )?PRIVATE KEY-----`, and similar. Full list is implementation-defined; see Phase 7.2.1.
- **If match found:** Warn the user and require explicit confirmation before writing. In non-interactive mode, deny the write and return an error.

---

## 6. Permission model semantics

- **Modes:** `Default` (prompt for state-changing tools), `AcceptAll` (no prompt; all tools allowed), `PlanOnly` (only Read, Glob, Grep; no Bash, Write, Edit), `DiffReview` (show a unified diff before each Write/Edit and require approval).
- **State-changing tools:** Write, Edit, Bash. Read, Glob, Grep are read-only.
- **Default behavior:** For state-changing tools, the permission checker returns `AskUser`. The CLI then prompts: allow once, always for session, or deny. In non-interactive mode (`--print` or no TTY), `AskUser` is treated as deny with an error message instructing the user to re-run with `--permission-mode accept-all`.
- **DiffReview mode:** Before each Write or Edit, a unified diff of the proposed change is shown. The user may Allow, Deny, or open the proposed content in an editor (EditInEditor). Useful for reviewing LLM-generated writes before they land on disk. Enable with `--permission-mode diff-review`.
- **Serialization:** Only one permission prompt is visible at a time (shared mutex across concurrent tool calls and subagents). No parallel stdin reads.
- **Allow/disallow lists:** Config can restrict which tools are allowed or disallowed; see [config.md](../schemas/config.md) `[tools]`.

Reference: [cli-interface-specification.md](../plans/cli-interface-specification.md) §7; [development-plan.md](../plans/development-plan.md) Phase 4.3.

---

## 7. Project-instruction trust model (CLIDO.md / CLAUDE.md)

- **Trust-on-first-use:** The first time Clido loads `CLIDO.md` or `CLAUDE.md` from a path (or the content hash changes), it prompts: "Load project instructions from {path}? [y/N]". If the user confirms, the path and content hash are stored in an allowlist (e.g. `{data_dir}/trusted_project_instructions.json`). If not, project instructions are skipped for that run.
- **Non-interactive:** In non-interactive mode, project instructions are loaded only if the path is already in the allowlist; otherwise they are skipped without prompting.
- **Size limit:** Project instruction files are capped (e.g. ~4,000 tokens). If exceeded, the file is truncated and a warning is logged. This limits cost and impact of adversarial or accidental huge files.
- **No auto-trust:** A repo cannot force its instructions to be loaded without user approval on first use.

Reference: [development-plan.md](../plans/development-plan.md) Phase 3.4.2.; security audit note in Phase 9.6.3 (project instructions prompt injection).

---

## 8. Sandbox behavior by OS

- **V1 (current):** Bash runs in the same environment as the Clido process. Sensitive env vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `AWS_*`, `GITHUB_TOKEN`, etc.) are stripped before spawning any shell command. Path traversal is prevented for file tools. No OS-level syscall sandbox at this tier.
- **V1.5 — `--sandbox` (macOS):** When `--sandbox` is set, Bash is wrapped in `sandbox-exec` with a profile that allows file reads, file writes only under `/tmp`, and outbound network connections. **Note:** `network-outbound` is still allowed in the current profile; this is a file-write guardrail, not full network isolation. `sandbox-exec` is also deprecated on macOS 14+; behavior is best-effort.
- **V1.5 — `--sandbox` (Linux):** When `--sandbox` is set and `bwrap` (Bubblewrap) is installed, Bash runs inside a mount namespace with `--unshare-net` (no network). Falls back to unsandboxed with a warning if `bwrap` is not found.
- **V2 target:** Full network isolation on macOS (drop `network-outbound` from the sandbox profile) and verified bwrap-based isolation on Linux. Docker sandbox (`--docker-sandbox`) as a future option.
- **Windows:** Bash is not guaranteed (no POSIX shell by default). When no shell is found, BashTool returns an error. No sandbox in V1/V2 for Windows; PowerShell or sandbox options are future work.

Reference: [development-plan.md](../plans/development-plan.md) Phase 7.1 and 9.5.2 (Windows).

---

## 9. Environment stripping for subprocesses

Before spawning any Bash (or MCP) subprocess, the following env vars are **removed** from the child environment unless explicitly allowlisted for that tool:

- `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GITHUB_TOKEN`, `AWS_*`, and other well-known secret-bearing names.
- Allowlist is configurable so that specific MCP servers can receive only the vars the user approves.

Reference: Phase 7.1.1, Phase 8.3 (MCP environment restriction).
