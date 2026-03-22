# Feature Plan: Web Fetch and Search Tools

## Status

Draft — not yet scheduled for a release milestone.

---

## Problem Statement

Real-world coding tasks constantly require consulting external information: crate documentation on docs.rs, API references on developer portals, Stack Overflow answers for error messages, GitHub Issues to understand why a library behaves unexpectedly, and release notes to understand breaking changes. Clido currently has zero internet tools. An agent that cannot look up information is forced to rely entirely on its training data, which is always stale, often incomplete for niche crates, and unable to verify that a crate version exists or an API signature is current.

This forces users into a disruptive loop: the agent suggests code, the user opens a browser tab, finds the right information, pastes it back into the chat, and continues. This is exactly the kind of mechanical work that a coding agent should eliminate.

---

## Competitive Analysis

| Tool | Web Tool | Search | API Key Required | HTML Extraction |
|------|----------|--------|-----------------|-----------------|
| Claude Code | `WebFetch` built-in | No native search | No | Basic |
| Cursor | Via agent mode | Bing integration | Yes (implicit) | Unknown |
| Cline | `browser_action` (Puppeteer) | No | No | Full DOM |
| Aider | None | None | N/A | N/A |
| **Clido (proposed)** | `WebFetch` + `WebSearch` | DuckDuckGo (default) / Brave (optional) | No default | `scraper` CSS extraction |

Clido's differentiators:
- No API key required for basic use (DuckDuckGo HTML endpoint).
- Higher-quality optional search via Brave API when the user provides a key.
- Smart HTML extraction using CSS selectors to remove navigation, footers, ads, and cookie banners — the agent receives clean markdown content, not raw HTML.
- `extract_code_blocks` option preserves code-heavy pages faithfully.
- Domain allowlist and blocklist for organizations that need to restrict the agent's internet access.
- `--no-web` flag for air-gapped environments and CI pipelines.
- TUI status strip shows the domain being fetched in real time.
- Rate limiting built in to prevent runaway tool loops from hammering external services.

---

## Design Decisions

### WebFetch

`WebFetch` fetches a single URL and returns the page content as markdown. It is a read-only tool (no side effects) and does not require a permission prompt in any `permission_mode`.

Input schema:
```json
{
  "url": "https://docs.rs/tokio/latest/tokio/",
  "max_chars": 8000,
  "extract_code_blocks": true
}
```

- `url` (required): the URL to fetch. Must be HTTP or HTTPS; `file://` and other schemes are rejected.
- `max_chars` (optional, default 8000): truncation limit for the returned content. Smart truncation: always include complete code blocks before truncating prose.
- `extract_code_blocks` (optional, default true): when true, `<pre><code>` blocks are extracted and preserved first before general prose extraction.

HTML-to-markdown extraction pipeline:
1. Fetch with `reqwest` (async, with timeout).
2. Parse HTML with `scraper` crate.
3. Remove elements matching: `nav`, `header`, `footer`, `.cookie-banner`, `#sidebar`, `[role="navigation"]`, `[role="complementary"]`, `.ads`, `.advertisement`, `script`, `style`.
4. Extract remaining text nodes and `<code>` / `<pre>` blocks.
5. Convert to markdown: headings preserved, inline code preserved, links preserved as `[text](url)` but only if the link is within the main content.
6. Collapse consecutive blank lines to a single blank line.
7. Apply `max_chars` truncation, preferring to cut between paragraphs.

Return format:
```
## Page Title

Source: https://docs.rs/tokio/latest/tokio/

<extracted markdown content>

[Truncated at 8000 chars — pass max_chars to increase limit]
```

### WebSearch

`WebSearch` returns a list of search results (title, snippet, URL). It does not fetch page content — the agent is expected to call `WebFetch` on relevant results.

Input schema:
```json
{
  "query": "tokio spawn blocking rust",
  "num_results": 5
}
```

- `query` (required): the search query string.
- `num_results` (optional, default 5, max 10): number of results to return.

Backend selection:
- **Default (no API key)**: DuckDuckGo HTML endpoint (`https://html.duckduckgo.com/html/?q=<query>`). Parse result titles, snippets, and URLs from the HTML response.
- **Brave API (if `CLIDO_BRAVE_API_KEY` env var or `[web] brave_api_key` config is set)**: use Brave Search API v1 (`https://api.search.brave.com/res/v1/web/search`). Higher quality, rate-limited per key.

Return format:
```
1. **tokio::task::spawn_blocking - Tokio Docs**
   https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
   Runs a blocking function on a dedicated thread pool. Use this for CPU-bound or I/O-blocking operations that would otherwise block the async runtime.

2. **How to use spawn_blocking in Tokio? - Stack Overflow**
   https://stackoverflow.com/questions/...
   ...
```

### Rate Limiting

To prevent tool loops from hammering external services:
- `WebFetch`: max 10 requests per agent turn. After the limit, the tool returns a rate-limit error message.
- `WebSearch`: max 5 requests per agent turn.
- Both limits are configurable via `[web] max_fetch_per_turn` and `[web] max_search_per_turn`.

These limits are tracked in-memory per agent turn; they reset at the start of each new turn.

### Domain Filtering

Organizations may need to restrict internet access. Config:

```toml
[web]
# If non-empty, only these domains (and subdomains) are allowed.
allowed_domains = ["docs.rs", "crates.io", "github.com", "stackoverflow.com"]

# Always blocked regardless of allowed_domains.
blocked_domains = ["internal.corp.example.com"]
```

When a URL is blocked, the tool returns an error message explaining the domain restriction.

### Permission Mode Integration

`WebFetch` is classified as a read-only tool. It never requires user confirmation regardless of `permission_mode`, consistent with `ReadFile` and `Glob`. `WebSearch` is also read-only and requires no confirmation. Neither tool modifies files or executes code.

---

## Implementation Plan

### New Crate: `clido-web`

Create `crates/clido-web/` with the following structure:

```
crates/clido-web/
  Cargo.toml
  src/
    lib.rs           — pub mod fetch; pub mod search; pub mod extractor; pub mod rate_limiter;
    fetch.rs         — WebFetchTool implementation
    search.rs        — WebSearchTool implementation
    extractor.rs     — HTML-to-markdown extraction logic
    rate_limiter.rs  — per-turn rate limiter
```

`Cargo.toml` dependencies:
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
scraper = "0.19"
tokio = { version = "1", features = ["rt"] }
url = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clido-core = { path = "../clido-core" }
clido-tools = { path = "../clido-tools" }
```

### `crates/clido-web/src/fetch.rs`

- `pub struct WebFetchTool { config: WebConfig, rate_limiter: Arc<RateLimiter> }`
- Implements `clido_tools::Tool` trait.
- `fn name() -> &'static str { "WebFetch" }`
- `fn description()` returns a description for the LLM explaining what the tool does.
- `fn input_schema()` returns the JSON Schema for `{url, max_chars?, extract_code_blocks?}`.
- `async fn call(&self, input: Value) -> Result<String, ToolError>`:
  1. Parse and validate `url` (must be http/https).
  2. Check domain against allowlist/blocklist.
  3. Check rate limiter; return error if exceeded.
  4. Fetch with `reqwest` (30-second timeout).
  5. Call `extractor::html_to_markdown()`.
  6. Truncate to `max_chars`.
  7. Return formatted string.

### `crates/clido-web/src/search.rs`

- `pub struct WebSearchTool { config: WebConfig, rate_limiter: Arc<RateLimiter> }`
- Implements `clido_tools::Tool` trait.
- `fn name() -> &'static str { "WebSearch" }`
- `async fn call(&self, input: Value) -> Result<String, ToolError>`:
  1. Parse `query` and `num_results`.
  2. Check rate limiter.
  3. If `config.brave_api_key` is set, call Brave API; otherwise call DuckDuckGo HTML endpoint.
  4. Parse results into `Vec<SearchResult>`.
  5. Format and return.

### `crates/clido-web/src/extractor.rs`

- `pub fn html_to_markdown(html: &str, extract_code_blocks: bool) -> String`
- Uses `scraper::Html::parse_document()` and `scraper::Selector`.
- Removal selectors applied before extraction.
- Code block extraction: select all `pre > code`, wrap in fenced markdown code blocks with detected language class.
- Prose extraction: walk remaining text nodes, preserving heading structure.

### `crates/clido-web/src/rate_limiter.rs`

- `pub struct RateLimiter { fetch_count: AtomicU32, search_count: AtomicU32, max_fetch: u32, max_search: u32 }`
- `pub fn check_and_increment_fetch(&self) -> Result<(), RateLimitError>`
- `pub fn check_and_increment_search(&self) -> Result<(), RateLimitError>`
- `pub fn reset(&self)` — called at start of each agent turn.

### Crate: `clido-core`

**Modify: `crates/clido-core/src/config_loader.rs`**

Add `[web]` section to `AgentConfig`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct WebConfig {
    pub brave_api_key: Option<String>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub max_fetch_per_turn: Option<u32>,   // default 10
    pub max_search_per_turn: Option<u32>,  // default 5
    pub no_web: bool,                       // default false
}
```

### Crate: `clido-agent`

**Modify: `crates/clido-agent/src/agent_loop.rs`**

- In the tool registry setup, instantiate `WebFetchTool` and `WebSearchTool` when `config.web.no_web` is false.
- Pass a shared `Arc<RateLimiter>` to both tools.
- Call `rate_limiter.reset()` at the start of each `run_next_turn()` invocation.
- Check `config.tools.disallowed` — if `"WebFetch"` or `"WebSearch"` is listed, skip registration.

### Crate: `clido-cli`

**Modify: `crates/clido-cli/src/cli.rs`**

- Add `--no-web` (`bool` flag) to `RunArgs`.
- Map to `AgentConfig.web.no_web = true`.

**Modify: `crates/clido-cli/src/tui.rs`**

- The tool event handler for `WebFetch` should update the status strip with: `Fetching: docs.rs...` (domain only, not full URL, to avoid clutter).
- The tool event handler for `WebSearch` should update the status strip with: `Searching: "<query>"...`.
- After completion, the event shows the result count or page title in the tool call history panel.

---

## Config Schema

```toml
[web]
# Brave Search API key for higher-quality search results.
# If unset, DuckDuckGo HTML endpoint is used (no key required).
brave_api_key = ""  # can also be set via CLIDO_BRAVE_API_KEY env var

# If non-empty, only these domains (and their subdomains) are accessible.
# Empty list means all domains are allowed.
allowed_domains = []

# Domains that are always blocked, even if they appear in allowed_domains.
blocked_domains = []

# Max WebFetch calls per agent turn (default: 10).
max_fetch_per_turn = 10

# Max WebSearch calls per agent turn (default: 5).
max_search_per_turn = 5

# Disable web tools entirely (equivalent to --no-web CLI flag).
no_web = false
```

---

## CLI Surface

```
clido run --no-web [...]
  # Disables WebFetch and WebSearch for this invocation.

clido doctor
  # Outputs web tool status:
  # [OK]  Web tools: enabled
  #         WebFetch: max 10/turn
  #         WebSearch: DuckDuckGo (no API key)
  # or:
  # [OK]  Web tools: enabled
  #         WebFetch: max 10/turn
  #         WebSearch: Brave API (key configured)
  # or:
  # [WARN] Web tools: disabled (--no-web)
```

---

## TUI Changes

### Status Strip Updates

During a tool call:
- `WebFetch` in progress: status strip shows `[web] Fetching docs.rs...`
- `WebFetch` complete: tool call history shows `WebFetch docs.rs/tokio/... (4,231 chars)`
- `WebSearch` in progress: status strip shows `[web] Searching "tokio spawn blocking"...`
- `WebSearch` complete: tool call history shows `WebSearch "tokio spawn blocking" → 5 results`

The domain is extracted from the URL using the `url` crate and displayed instead of the full URL to avoid overflow in narrow terminals.

---

## Test Plan

### Unit Tests — `crates/clido-web/src/`

1. **`test_fetch_valid_url`** — mock HTTP server (mockito) returns simple HTML; assert markdown output contains the main text.
2. **`test_fetch_strips_navigation`** — HTML with `<nav>` and `<footer>` elements; assert those elements' text does not appear in output.
3. **`test_fetch_preserves_code_blocks`** — HTML with `<pre><code class="language-rust">...</code></pre>`; assert fenced code block with `rust` language tag appears in output.
4. **`test_fetch_max_chars_truncation`** — content exceeds `max_chars`; output length is at most `max_chars + some_overhead`; truncation message is appended.
5. **`test_fetch_non_http_scheme_rejected`** — input `file:///etc/passwd`; tool returns an error, no HTTP request is made.
6. **`test_fetch_blocked_domain`** — domain appears in `blocked_domains`; tool returns a domain-blocked error message.
7. **`test_fetch_allowed_domains_enforced`** — `allowed_domains = ["docs.rs"]`; fetching `github.com` returns a domain-not-allowed error.
8. **`test_fetch_rate_limit`** — call `WebFetch` 11 times in the same turn; 11th call returns rate limit error.
9. **`test_rate_limiter_resets_between_turns`** — call 10 times, call `reset()`, call again; no rate limit error.
10. **`test_search_ddg_parsing`** — mock DuckDuckGo HTML response; assert 5 results returned with title, url, snippet.
11. **`test_search_brave_api`** — mock Brave API JSON response; assert results are parsed correctly.
12. **`test_search_rate_limit`** — call `WebSearch` 6 times; 6th call returns rate limit error.
13. **`test_html_extractor_heading_structure`** — HTML with `h1`, `h2`, `p`; assert markdown heading levels are preserved.
14. **`test_no_web_flag_prevents_tool_registration`** — `AgentConfig { web: WebConfig { no_web: true, .. } }`; tool registry does not contain `WebFetch` or `WebSearch`.

### Integration Tests — `crates/clido-agent/tests/`

15. **`test_agent_uses_web_fetch_in_tool_loop`** — mock HTTP server + mock LLM that returns a `WebFetch` tool call; assert the tool call is executed and the result is fed back to the LLM.

---

## Docs Updates

- **New file**: `docs/guide/web-tools.md` — covers `WebFetch` and `WebSearch` tool descriptions, examples of when the agent uses them, how to configure Brave API key, how to use `allowed_domains` for restricted environments, and the `--no-web` flag.
- **Update**: `docs/reference/cli-flags.md` — add `--no-web` entry.
- **Update**: `docs/reference/configuration.md` — add the full `[web]` section with all keys documented.
- **Update**: `docs/guide/tools.md` (or equivalent tool reference) — add `WebFetch` and `WebSearch` tool entries with input schema tables.
- **Update**: `docs/developer/architecture.md` — mention `clido-web` crate and its role.

---

## Definition of Done

- [ ] `clido-web` crate builds cleanly with `cargo build` and `cargo clippy -- -D warnings` passes with no warnings.
- [ ] `WebFetch` correctly fetches a real URL (integration smoke test in CI, gated behind a `#[cfg(feature = "network-tests")]` feature flag to keep default test runs offline).
- [ ] HTML extraction removes nav, footer, sidebar, and ad elements from at least 3 real-world sites (verified manually and encoded as test fixtures).
- [ ] `WebSearch` returns results from DuckDuckGo without requiring any API key.
- [ ] Brave API search works when `CLIDO_BRAVE_API_KEY` is set (verified manually or via CI secret).
- [ ] Rate limiting prevents more than `max_fetch_per_turn` / `max_search_per_turn` calls in a single turn; the counter resets at turn start.
- [ ] `--no-web` flag prevents both `WebFetch` and `WebSearch` from being registered in the tool loop.
- [ ] Domain allowlist and blocklist are enforced; a blocked URL returns a user-readable error, not a panic.
- [ ] TUI status strip shows the domain name (not full URL) during a `WebFetch` call and the query during a `WebSearch` call.
- [ ] `clido doctor` reports web tool status including which search backend is active.
- [ ] All 15 tests listed in the test plan pass with `cargo test`.
- [ ] `docs/guide/web-tools.md` is written and linked from the VitePress sidebar.
- [ ] Configuration reference is updated with the `[web]` section.
