# Feature Plan: Desktop Notifications and Completion Hooks

**Status:** Planned
**Target release:** V2
**Crate(s) affected:** `clido-cli`, `clido-core`
**New files:** `crates/clido-cli/src/notify.rs`, `docs/guide/notifications.md`

---

## 1. Problem Statement

When Clido runs a long task — file indexing, a multi-step refactor, a large codebase search — the user has no choice but to keep the terminal in view and watch for the prompt to return. Tasks that run 2–5 minutes are common. During that time the user cannot focus on another window without losing their place.

Claude Code sends a macOS notification via its Electron shell when a task completes, but this is a side effect of running in a browser process rather than a deliberate multi-channel notification design. It does not support webhooks, does not include cost or duration in the notification body, and has no minimum-duration gate (fires even for 1-second tasks, creating noise). It also has no terminal bell option for headless / SSH workflows.

Clido runs entirely in the terminal. Without an explicit notification layer, it is blind to whether the user is present. This breaks flow for anyone working in a split-screen setup, a tmux session, or across multiple monitors.

---

## 2. Competitive Analysis

| Tool | Desktop notif | Terminal bell | Webhook | Includes cost | Min-duration gate |
|------|--------------|---------------|---------|--------------|-------------------|
| Claude Code | Yes (macOS only, via Electron) | No | No | No | No |
| Cursor | No (inline only) | No | No | No | No |
| Cline (VS Code) | Via VS Code API | No | No | No | No |
| Aider | No | Optional | No | No | No |
| **Clido (this plan)** | **Yes (macOS + Linux + Win future)** | **Yes** | **Yes (Slack/generic)** | **Yes** | **Yes** |

Clido's notification system is the most complete in its class because it addresses three distinct user contexts: desktop (GUI notification), terminal (bell), and remote/async (webhook). No other open-source coding agent in this category supports webhooks for task completion.

---

## 3. Our Specific Improvements

### 3.1 Multi-channel delivery
Notifications fire via up to three channels simultaneously:
1. OS desktop notification (platform-adaptive, no compiled dependency)
2. Terminal bell (`\x07` to stderr)
3. HTTP POST webhook (Slack-compatible JSON or generic JSON)

Each channel is independently toggleable in config.

### 3.2 Notification body includes task context
The notification title is `"Clido: task complete"` (or `"Clido: task failed"` on error). The body is:
```
<first 80 chars of prompt>…
Duration: 2m 14s  |  Cost: $0.0031
```
No other tool includes cost in its completion notification.

### 3.3 Focus-aware firing
Clido detects whether the terminal window is likely focused before firing OS notifications:
- If `TERM_PROGRAM=iTerm.app` is set, iTerm2's focus reporting is checked via `ITERM_PROFILE` presence heuristics.
- Fallback: compare `WINDOWID` (X11) or check for `TERM_SESSION_ID` (macOS Terminal.app).
- If detection is inconclusive, default is to fire. Users can override with `--notify` (always fire) or `--no-notify` (never fire).

### 3.4 Minimum-duration gate
`min_duration_secs` prevents notification spam for sub-10-second tasks. The default is 10 seconds, meaning quick completions are silent. Long tasks that justify a notification always fire.

### 3.5 Sound support
Optional sound effect on completion. macOS uses `afplay` or the `osascript` sound parameter. Linux uses `paplay`/`aplay` if available. The path is configurable so users can supply a custom WAV/MP3.

---

## 4. Design Decisions and Rationale

**No compiled native dependencies for OS notifications.**
Using `osascript` (macOS) and `notify-send` (Linux) means zero additional `Cargo.toml` dependencies for the core notification path. This keeps compile times fast and avoids C library linking issues in CI. The tradeoff is that these are subprocess calls (forked, not blocking), which is acceptable because notifications are fire-and-forget.

**Webhook fires asynchronously using `tokio::spawn`.**
The webhook POST must not block the CLI from returning to the prompt. A failed webhook (non-2xx or timeout) logs a warning to stderr but does not propagate as an error.

**Notification fires at `AgentEvent::Response` or `AgentEvent::Err`, not at process exit.**
Hooking into the `AgentEvent` stream rather than a process signal means the notification carries the full context (cost, turn count) that is only available inside the agent loop. Process-exit hooks via `atexit` cannot access this data.

**Separate `notify.rs` module, not inlined in `run.rs`.**
The notification logic is self-contained and testable in isolation. It receives a `NotificationPayload` struct and knows nothing about the agent loop internals.

**`[notifications]` config section rather than top-level keys.**
Groups related settings, reduces config clutter, and makes the section easy to document and validate independently.

---

## 5. Implementation Steps

### 5.1 New struct: `NotificationPayload`

File: `crates/clido-cli/src/notify.rs`

```rust
pub struct NotificationPayload {
    pub prompt_summary: String,   // first 80 chars of original prompt
    pub duration_ms: u64,
    pub cost_usd: f64,
    pub exit_status: ExitStatus,  // enum: Completed | Failed | Interrupted
    pub session_id: String,
}
```

### 5.2 `NotificationConfig` in `clido-core`

Add to `crates/clido-core/src/config_loader.rs` (or new `notification_config.rs`):

```toml
[notifications]
enabled = true
on_completion = true
on_error = true
min_duration_secs = 10
webhook_url = ""
bell = false
sound = false
sound_path = ""        # optional: path to WAV/MP3
focus_aware = true     # suppress if terminal appears focused
```

Deserialize into `NotificationConfig` struct with `serde`. Add to `AgentConfig` as `pub notifications: NotificationConfig`.

### 5.3 Platform backends in `notify.rs`

```
fn notify_desktop(payload: &NotificationPayload) -> Result<()>
  -> try macos_notify() first
  -> else try linux_notify()
  -> else try windows_notify()   // future
  -> if all fail, log debug-level warning and return Ok(())

fn macos_notify(title: &str, body: &str) -> Result<()>
  -> spawn osascript -e 'display notification ...'

fn linux_notify(title: &str, body: &str) -> Result<()>
  -> spawn notify-send title body

fn bell_notify()
  -> write \x07 to stderr

async fn webhook_notify(url: &str, payload: &NotificationPayload) -> Result<()>
  -> reqwest POST with JSON body (see §5.5)
  -> timeout 5s
```

### 5.4 Integration point in `run.rs`

In `crates/clido-cli/src/run.rs`, after the agent loop returns, construct `NotificationPayload` from `AgentLoopResult` and call:

```rust
notification::fire(&cfg.notifications, &payload).await;
```

`fire()` checks `min_duration_secs`, `enabled`, and each channel config before dispatching.

### 5.5 Webhook JSON body

```json
{
  "prompt":      "<first 80 chars of prompt>",
  "duration_ms": 134500,
  "cost_usd":    0.0031,
  "exit_status": "completed",
  "session_id":  "sess_abc123"
}
```

Slack incoming webhooks accept `{"text": "..."}` — detect Slack URL by checking for `hooks.slack.com` in the URL and wrap accordingly.

### 5.6 CLI flags

In `crates/clido-cli/src/cli.rs`:

```
--notify        Force-enable notifications for this run (ignores min_duration_secs, ignores focus state)
--no-notify     Suppress all notifications for this run
```

Implemented as a tristate in `RunArgs`: `notify: Option<bool>`, overriding `NotificationConfig::enabled`.

### 5.7 TUI status bar indicator

In `crates/clido-cli/src/tui.rs`, the status bar right-side currently shows model name and cost. Add a notification indicator character (bell icon or `[N]` in non-Unicode terminals) when `notifications.enabled = true`. Toggle reactively if user changes config at runtime (future: `/notify` slash command to toggle).

---

## 6. Config Schema (complete)

```toml
[notifications]
# Master switch. When false, all channels are suppressed.
enabled = true

# Fire on successful task completion.
on_completion = true

# Fire when the agent exits with an error or is interrupted.
on_error = true

# Only fire if the task took at least this many seconds.
# Set to 0 to always fire.
min_duration_secs = 10

# HTTP endpoint for webhook delivery. Empty string = disabled.
# Slack incoming webhook URLs (hooks.slack.com) are detected automatically.
webhook_url = ""

# Send terminal bell character (\x07) to stderr on completion.
bell = false

# Play a sound on completion. Requires afplay (macOS) or paplay/aplay (Linux).
sound = false

# Path to a WAV or MP3 file for the sound effect.
# When empty, uses the platform default notification sound.
sound_path = ""

# Suppress OS notification if the terminal appears to be focused.
# Set to false to always fire regardless of focus state.
focus_aware = true
```

---

## 7. CLI Surface

```
clido run "refactor auth module" --notify
clido run "search for dead code" --no-notify
```

`--notify` and `--no-notify` are mutually exclusive (clap conflict group).

---

## 8. TUI Changes

- Status bar: append `[notif:on]` or `[notif:off]` (or bell glyph in capable terminals) to the right side of the bottom bar.
- When a notification fires (OS or webhook), emit an `AgentEvent::Notification` (new variant) so the TUI can show a brief inline message: `  notification sent`.
- No new slash commands in this release. (Future: `/notify on|off` toggle.)

---

## 9. New `AgentEvent` Variant

Add to the `AgentEvent` enum in `tui.rs`:

```rust
Notification {
    channel: NotificationChannel, // Desktop | Bell | Webhook
    success: bool,
}
```

The TUI renders this as a dim info line in the chat transcript.

---

## 10. Test Plan

### Unit tests — `crates/clido-cli/src/notify.rs`

**`test_notification_message_format`**
Given `prompt = "Refactor the authentication module to use JWT"`, `duration_ms = 134500`, `cost_usd = 0.0031`:
- Assert title equals `"Clido: task complete"`.
- Assert body contains `"Refactor the authentication module to use JWT"`.
- Assert body contains `"2m 14s"`.
- Assert body contains `"$0.0031"`.

**`test_prompt_truncation_at_80_chars`**
Given a 200-character prompt string, assert `prompt_summary` is exactly 80 characters followed by `"…"`.

**`test_min_duration_gate_suppresses_short_task`**
Given `min_duration_secs = 10`, `duration_ms = 5000`: assert `fire()` returns without calling any backend.

**`test_min_duration_gate_passes_long_task`**
Given `min_duration_secs = 10`, `duration_ms = 15000`: assert at least one backend is attempted.

**`test_notify_flag_overrides_min_duration`**
Given `min_duration_secs = 10`, `duration_ms = 3000`, `force = true`: assert notification fires.

**`test_webhook_payload_serialization`**
Serialize a `WebhookPayload` to JSON and assert all fields present with correct types.

**`test_slack_url_detection`**
Given `url = "https://hooks.slack.com/services/T00/B00/xxx"`: assert Slack wrapping is applied (body has `"text"` key).

**`test_generic_webhook_url_detection`**
Given `url = "https://example.com/webhook"`: assert generic JSON body is used (no `"text"` key wrapping).

### Unit tests — `crates/clido-core/src/config_loader.rs`

**`test_notification_config_defaults`**
Parse an empty `[notifications]` section and assert all defaults match the values in §6.

**`test_notification_config_override`**
Parse a TOML string with `bell = true` and `min_duration_secs = 30`, assert fields are set correctly.

### Integration tests — `crates/clido-cli/tests/notify_integration.rs`

**`test_notify_send_invocation_mock`**
Mock the `notify-send` binary with a shell script that writes args to a temp file. Run `fire()` on Linux path. Assert the temp file contains the expected title and body.

**`test_webhook_post_mock`**
Use `wiremock` or `httpmock` crate to stand up a local HTTP server. Call `webhook_notify()` with the mock URL. Assert the server received exactly one POST with correct JSON body.

---

## 11. Docs Pages

### New: `docs/guide/notifications.md`
Full guide covering: enabling notifications, platform support, configuring the webhook, Slack integration walkthrough, SSH/headless usage tips (`bell = true`, `focus_aware = false`), minimum duration tuning.

### Update: `docs/reference/configuration.md`
Add `[notifications]` section table with all keys, types, defaults, and descriptions.

### Update: `docs/reference/flags.md`
Add `--notify` and `--no-notify` to the flags reference table.

### Update: `docs/guide/tui.md`
Add screenshot / ASCII art showing the notification indicator in the status bar.

---

## 12. Definition of Done

- [ ] `crates/clido-cli/src/notify.rs` exists with `NotificationPayload`, `NotificationConfig`, `fire()`, and all platform backend functions.
- [ ] `NotificationConfig` is deserialized from `~/.config/clido/config.toml` and present on `AgentConfig`.
- [ ] macOS desktop notification fires via `osascript` subprocess when task completes and duration exceeds `min_duration_secs`.
- [ ] Linux desktop notification fires via `notify-send` when available; silently skipped when not installed.
- [ ] Terminal bell (`\x07`) fires when `bell = true` in config.
- [ ] Webhook POST fires with correct JSON body (all fields from §5.5 present); Slack URL detection wraps body in `{"text": ...}`.
- [ ] `--notify` CLI flag forces notification regardless of duration or focus state.
- [ ] `--no-notify` CLI flag suppresses all channels for the run.
- [ ] Notification indicator visible in TUI status bar when `enabled = true`.
- [ ] New `AgentEvent::Notification` variant handled in TUI without panic.
- [ ] All 10 unit and integration tests pass in CI.
- [ ] `docs/guide/notifications.md` written and linked from sidebar.
- [ ] `docs/reference/configuration.md` updated with `[notifications]` section.
- [ ] `docs/reference/flags.md` updated with `--notify` and `--no-notify`.
- [ ] No new `Cargo.toml` dependencies required for the core notification path (only `reqwest` for webhook, which is already a dependency).
