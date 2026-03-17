# Config and pricing reference

This document is the **field-by-field reference** for all Clido configuration files: `config.toml`, `.clido/config.toml`, and `pricing.toml`. Every key is documented with type, default, env override (if any), and validation.

**Canonical env and flag names:** Config path is overridden by `CLIDO_CONFIG` (not `CLIDO_CONFIG_FILE`). Budget flag is `--max-budget-usd` (not `--max-budget`). Log level is `CLIDO_LOG` (not `RUST_LOG`). See [cli-interface-specification.md](../plans/cli-interface-specification.md) for the full list.

---

## 1. config.toml (and .clido/config.toml)

**Config load order:** (1) Built-in defaults, (2) Global config file, (3) Project config file (`.clido/config.toml` in cwd or nearest ancestor), (4) CLI flags. Later steps override earlier for overlapping keys.

**Global config path:** `~/.config/clido/config.toml` (or `$CLIDO_CONFIG` if set to a file path; the env var overrides the default path, not merge).

---

## Top-level keys

| Key | Type | Default | Env override | Validation |
|-----|------|---------|--------------|------------|
| `default_profile` | string | `"default"` | — | Must match a key in `[profile.<name>]`. If missing, error: "Profile '…' not found. Check default_profile in config." |

---

## [profile.<name>]

Each profile is a table under `[profile.<name>]`. The name is the profile identifier (e.g. `default`, `cheap`, `local`).

| Key | Type | Default | Env override | Validation |
|-----|------|---------|--------------|------------|
| `provider` | string | — | — | One of: `anthropic`, `openai`, `openrouter`, `alibaba`, `local`. Else: "Unknown provider '…'. Valid: anthropic, openai, openrouter, alibaba, local." |
| `model` | string | — | `CLIDO_MODEL` | Non-empty. |
| `api_key_env` | string | — | — | Name of env var to read API key from (e.g. `ANTHROPIC_API_KEY`). Required for non-local providers. If set and env var empty/absent at startup: "Error: API key not found for profile '…'. Set … or run clido doctor." |
| `base_url` | string | provider-specific | — | URL for API. For `local`, default `http://localhost:11434/v1`. Must be valid URL. |

**Example:**

```toml
[profile.default]
provider = "anthropic"
model = "claude-sonnet-4-5"
api_key_env = "ANTHROPIC_API_KEY"
base_url = "https://api.anthropic.com"
```

---

## [agent]

| Key | Type | Default | Env override | Validation |
|-----|------|---------|--------------|------------|
| `max_turns` | integer | 50 | `CLIDO_MAX_TURNS` | > 0, ≤ 1000. |
| `max_budget_usd` | float | 5.0 | `CLIDO_MAX_BUDGET_USD` | ≥ 0. |
| `max_concurrent_tools` | integer | 10 | — | > 0, ≤ 100. Semaphore cap for read-only tool concurrency. |

---

## [tools]

| Key | Type | Default | Env override | Validation |
|-----|------|---------|--------------|------------|
| `allowed` | array of string | `[]` | — | Empty = all allowed. Otherwise only listed tools are allowed. |
| `disallowed` | array of string | `[]` | — | Tool names to deny. Takes precedence over allowed if both set. |

---

## [context]

| Key | Type | Default | Env override | Validation |
|-----|------|---------|--------------|------------|
| `compaction_threshold` | float | 0.75 | — | In (0, 1]. Compact when context_tokens > max_context_tokens * compaction_threshold. |
| `max_context_tokens` | integer | model-dependent | — | From pricing/context_window or default (e.g. 200000). |

---

## [provider]

Global provider-related options (not per-profile).

| Key | Type | Default | Env override | Validation |
|-----|------|---------|--------------|------------|
| `offline` | boolean | false | — | If true, no external network; see devdocs/guides/pricing-and-offline.md. |

---

## Project-level config (`.clido/config.toml`)

Same schema as above. Only the keys present are merged; absent keys inherit from the global config. Merge is shallow: a full `[profile.xyz]` table in project config replaces the global `[profile.xyz]` for that profile, not field-by-field merge (unless the implementation chooses field-level merge for profiles; the spec allows either as long as it is documented).

**Location:** Walk from cwd upward until `$HOME` or filesystem root; use the first directory containing `.clido/config.toml`.

---

## Error messages (summary)

- Unknown profile: "Profile '…' not found. Check default_profile in config."
- Missing API key: "Error: API key not found for profile '…'. Set <env_var> or run clido doctor."
- Unknown provider: "Unknown provider '…'. Valid: anthropic, openai, openrouter, alibaba, local."
- Invalid number (e.g. max_turns): "Invalid value for …: expected positive integer."
- Invalid compaction_threshold: "Invalid value for context.compaction_threshold: expected number in (0, 1]."

All config errors are reported at startup before any API or tool execution.

---

## 2. pricing.toml

**File location:** User override: `{config_dir}/clido/pricing.toml` (e.g. `~/.config/clido/pricing.toml`). If absent, the shipped default (embedded or `clido-providers/data/pricing.toml`) is used.

**Per-model table** (e.g. `[model.claude-sonnet-4-5]`):

| Key | Type | Required | Default / notes |
|-----|------|----------|-----------------|
| `name` | string | yes | Display name (e.g. `claude-sonnet-4-5`). |
| `provider` | string | yes | `anthropic`, `openai`, `openrouter`, `alibaba`, `local`. |
| `input_per_mtok` | float | yes | USD per million input tokens. |
| `output_per_mtok` | float | yes | USD per million output tokens. |
| `cache_creation_per_mtok` | float | no | Default 1.25 × input_per_mtok. |
| `cache_read_per_mtok` | float | no | Default 0.10 × input_per_mtok. |
| `context_window` | integer | no | Model context size in tokens. |

If a requested model is missing from the table, Clido logs a warning and uses a configurable default pricing entry. Staleness: if the file is older than 90 days, a startup warning is emitted; `clido doctor` reports it as a warning.

Full schema, `update-pricing` source of truth, and examples: [pricing-and-offline.md](../guides/pricing-and-offline.md).
