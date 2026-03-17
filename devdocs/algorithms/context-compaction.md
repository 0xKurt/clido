# Context compaction algorithm

This document defines the trigger threshold, compaction prompt template, fallback and hard-limit behavior, provider normalization mapping, retry policy, and tool concurrency bounds.

---

## 1. Compaction trigger and threshold

- **Trigger:** Compaction runs when `context_tokens > max_context_tokens * compaction_threshold`.
- **Default threshold:** `0.75` (configurable via `[context] compaction_threshold = 0.75` in config).
- **max_context_tokens:** Comes from the model's context window (e.g. from pricing.toml `context_window` or provider default). Example: 200000 for Claude.

When the condition holds, the context engine invokes the compaction step before sending the next request to the model.

---

## 2. What is preserved verbatim

- **Pinned messages:** All system-role messages injected by the context builder (project instructions, CLIDO.md, tool guidance) are skipped by compaction and re-inserted in full above the compaction summary. User messages are never pinned.
- **Most recent tool results:** The last N turns' worth of tool results are kept in full (default: last 2 turns). This avoids dropping the immediate context the model needs to continue.
- **Compaction output:** Oldest user/assistant/tool history in the compacted range is replaced by a single system message: `"[Compacted history] {summary}"`. The summary is produced by a separate summarization model call (see template below).

---

## 3. Compaction prompt template

The summarization call uses a dedicated system prompt. The following is the exact template (one possible formulation; implement to this effect):

```
You are a summarizer for a coding agent session. Summarize the following conversation history in 2–4 concise paragraphs. Preserve:
- Every file path that was read or edited (list them).
- Every tool name that was called (list them).
- The user's high-level goal and any constraints they stated.
- The current state of the task (what was done, what might be left).

Output only the summary, no preamble.
```

The input to this call is the raw text of the messages being compacted (roles and content flattened). The model response is the summary string inserted as `[Compacted history] {summary}`.

---

## 4. Fallback path

- If the **summarization provider call fails** for any reason (network error, 5xx, timeout, malformed response): log `warn!` with the error, **skip compaction for this turn**, and continue the agent loop. Do not halt the agent. The next turn may again exceed the threshold and retry compaction.
- If **context_tokens > hard_limit** (the model's maximum context window) and compaction is skipped (e.g. summarization failed): do **not** send a request that would exceed the hard limit. Halt the turn and emit a user-visible error: "Context is too large to compact and too large to send ({n} tokens > {hard_limit}). Start a new session or use --context-limit to configure a smaller limit."

---

## 5. Provider normalization mapping

Exact field name and structure translation between provider APIs and Clido's internal `Message` / `ContentBlock` types.

### Anthropic → Clido

| Anthropic (API) | Clido (internal) |
|-----------------|------------------|
| `role: "user"` | `Message.role = "user"` |
| `content[].type: "text"`, `content[].text` | `ContentBlock::Text { text }` |
| `content[].type: "tool_use"`, `id`, `name`, `input` | `ContentBlock::ToolUse { id, name, input }` |
| User block with `content[].type: "tool_result"`, `tool_use_id`, `content`, `is_error` | `ContentBlock::ToolResult { tool_use_id, content, is_error }` |
| `stop_reason` | `ModelResponse.stop_reason` (map to StopReason) |
| `usage.input_tokens`, `usage.output_tokens`, `usage.cache_creation_input_tokens`, `usage.cache_read_input_tokens` | `Usage` struct |

### OpenAI / OpenRouter → Clido

| OpenAI (API) | Clido (internal) |
|--------------|------------------|
| `messages[].role: "user"`, `messages[].content` (string or array) | `Message.role = "user"`, content blocks from content array or single text |
| `messages[].role: "assistant"`, `messages[].content`, `messages[].tool_calls[]` | `ContentBlock::Text` from content; `ContentBlock::ToolUse` from each `tool_calls[]` with `id`, `function.name` → `name`, `function.arguments` (parsed JSON) → `input` |
| `messages[].role: "tool"`, `tool_call_id`, `content` | `ContentBlock::ToolResult { tool_use_id, content, is_error }` (is_error from content or convention) |
| `choices[0].finish_reason` | `StopReason` |
| `usage.prompt_tokens`, `usage.completion_tokens` | `Usage.input_tokens`, `Usage.output_tokens`; cache fields if provided |

---

## 6. Retry policy (provider HTTP calls)

- **Max attempts:** 3 (initial + 2 retries).
- **Backoff:** Exponential: 1s, 2s, 4s. Use `tokio::time::sleep` between attempts.
- **Retry-After:** If the response has a `Retry-After` header (seconds or HTTP-date), use it instead of the next backoff value for that attempt.
- **Request timeout:** 120 seconds per attempt.
- **Retry on:** HTTP 429 (rate limit), HTTP 5xx. Log each retry at `warn!`.
- **Do not retry on:** 400, 401, 403, 404, or any other 4xx. Return `ClidoError::Provider` immediately with the response body (redacted if it might contain secrets).

---

## 7. Tool concurrency bounds

- **Semaphore cap:** Default **10** concurrent tool executions (configurable via `[agent] max_concurrent_tools = 10`).
- **Read-only tools:** Up to `max_concurrent_tools` may run in parallel (via `join_all` or similar), bounded by the semaphore.
- **State-changing tools:** Always executed sequentially, in the order they appear in the model response. They do not run in parallel with each other.
- **Mixed batch:** When the model returns both read-only and state-changing calls: execute all read-only first (concurrently, under the semaphore), collect results; then execute state-changing tools one at a time in order; return all results to the model in the original tool-call order.

**Rationale for 10:** Keeps parallelism useful for read-heavy turns while avoiding filesystem descriptor exhaustion on typical OS defaults (e.g. 1024 open files per process). Adjust in config for larger machines if needed.
