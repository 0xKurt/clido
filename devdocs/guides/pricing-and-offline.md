# Pricing and Offline Mode

This guide defines the `pricing.toml` schema, the source of truth for `clido update-pricing`, and the consolidated behavior of `offline = true`.

---

## 1. pricing.toml schema

### File location

- **User override:** `{config_dir}/clido/pricing.toml` (e.g. `~/.config/clido/pricing.toml` on Linux/macOS).
- **Shipped default:** Embedded in the binary or in `clido-providers/data/pricing.toml`; used when no user file exists.

At startup, Clido loads pricing: first look for the user override; if absent, use the shipped default. This allows users to update prices when providers change them without recompiling.

### Per-model structure

Each entry is a TOML table under the `[model.<key>]` or equivalent array-of-tables format. The following fields are defined:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Display name for the model (e.g. `claude-sonnet-4-5`). |
| `provider` | string | yes | Provider identifier: `anthropic`, `openai`, `openrouter`, `alibaba`, `local`. |
| `input_per_mtok` | float | yes | Price per million input tokens (USD). |
| `output_per_mtok` | float | yes | Price per million output tokens (USD). |
| `cache_creation_per_mtok` | float | no | Price per million cache-creation input tokens (Anthropic). Default: 1.25 × `input_per_mtok`. |
| `cache_read_per_mtok` | float | no | Price per million cache-read input tokens (Anthropic). Default: 0.10 × `input_per_mtok`. |
| `context_window` | integer | no | Model context window size in tokens. Used for context limit validation. |

If a model is requested that is not in the pricing table, Clido logs a warning and falls back to a configurable `default_pricing` entry rather than erroring, so cost-tracking failure does not block the agent.

### Example

```toml
# pricing.toml — per-model pricing (USD per million tokens)

[model.claude-sonnet-4-5]
name = "claude-sonnet-4-5"
provider = "anthropic"
input_per_mtok = 3.0
output_per_mtok = 15.0
cache_creation_per_mtok = 3.75
cache_read_per_mtok = 0.30
context_window = 200000

[model.gpt-4o]
name = "gpt-4o"
provider = "openai"
input_per_mtok = 2.5
output_per_mtok = 10.0
context_window = 128000
```

---

## 2. update-pricing source of truth

The `clido update-pricing` command (V1.5+) fetches a canonical pricing file from a **versioned URL** controlled by the clido project.

- **URL:** A pinned URL to a file in the clido GitHub repository, e.g. `https://raw.githubusercontent.com/<org>/clido/main/dist/pricing.json` (or a tagged release path for stability).
- **Format fetched:** JSON (not TOML) for easy generation and validation. The command validates the JSON schema, then converts or merges into the user's `pricing.toml` at `{config_dir}/clido/pricing.toml`.
- **Offline behavior:** If the network is unavailable, `update-pricing` must fail gracefully with a clear message (e.g. "Network unavailable. Check your connection or run without offline mode."). It must never overwrite or corrupt the existing file on failure. Implementation: download to a temp file, validate, then rename atomically into place.

The initial/default `pricing.toml` that ships with Clido is maintained in the repo (e.g. `dist/pricing.toml` or equivalent) and can be generated from the same source as `dist/pricing.json` for consistency.

---

## 3. offline = true — consolidated behavior

When `offline = true` is set (in config or for a profile), Clido must **never** attempt to reach any external host. All of the following apply in one place:

| Component | Behavior when `offline = true` |
|-----------|--------------------------------|
| **Model API** | All model API calls go to localhost only. If the selected profile's `base_url` (or equivalent) is not `localhost` or `127.0.0.1`, Clido emits an error at startup: "offline = true but provider points to '{url}'. Set provider to a local endpoint or configure a localhost base_url." |
| **Telemetry / analytics** | Disabled completely; no pings. |
| **clido update-pricing** | Skipped. If the user runs `clido update-pricing`, the command exits with a clear message: "Offline mode is enabled. Connect to the network to update pricing." No network call is attempted. |
| **Embedding model (fastembed)** | If the ONNX model cache is absent (e.g. first run without prior download), `embed()` must fail with a clear message: "Embedding model not cached. Run with network access first (e.g. run 'clido fetch-models' or disable offline mode), then switch to offline mode." Do not silently fail or produce wrong embeddings. |
| **clido doctor** | Any check that requires network (e.g. provider connectivity ping) is skipped and reported as `[skipped — offline mode]`, not as a failure. |

**Startup check:** If `offline = true` and the selected profile points to a non-local host, Clido must refuse to start and print the error above.

**CI:** Tests that use mock HTTP servers (e.g. wiremock) are fine in offline mode. Tests that require real network access must be gated (e.g. `#[cfg(feature = "integration")]`) and excluded from the default CI matrix.

Document this in the user-facing README under "Offline / Air-gapped Usage."
