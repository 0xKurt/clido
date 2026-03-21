# Adding Providers

This page explains how to add a new LLM provider to clido.

## The `ModelProvider` trait

All providers implement the `ModelProvider` trait defined in `crates/clido-providers/src/lib.rs`:

```rust
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Display name (used in UI and logs).
    fn name(&self) -> &str;

    /// Send a blocking (non-streaming) completion request.
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &AgentConfig,
    ) -> Result<ModelResponse>;

    /// Stream a completion (optional; falls back to complete() if not implemented).
    async fn stream_complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &AgentConfig,
    ) -> Result<ModelResponse> {
        // Default implementation: delegate to complete()
        self.complete(messages, tools, config).await
    }
}
```

The core types (`Message`, `ContentBlock`, `ToolSchema`, `ModelResponse`, etc.) are defined in `clido-core`.

## Message and response types

### `Message`

```rust
pub struct Message {
    pub role: Role,                  // Role::User or Role::Assistant
    pub content: Vec<ContentBlock>,
}

pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
    // ... cache control variants for Anthropic
}
```

### `ModelResponse`

```rust
pub struct ModelResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
    pub model: String,
}

pub enum StopReason {
    EndTurn,    // Agent finished responding
    ToolUse,    // Agent wants to call tools
    MaxTokens,  // Context window full
    StopSequence,
}

pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,   // Anthropic only
    pub cache_creation_input_tokens: Option<u32>, // Anthropic only
}
```

## Worked example: adding a "Gemini" provider

Here is a complete example of adding Google Gemini support (hypothetical — for illustration).

### Step 1: Create the provider file

Create `crates/clido-providers/src/gemini.rs`:

```rust
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use clido_core::{
    AgentConfig, ContentBlock, Message, ModelResponse, Result, Role, StopReason,
    ToolSchema, Usage, ClidoError,
};

use crate::ModelProvider;

pub struct GeminiProvider {
    api_key: String,
    client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

// ── Wire format (Gemini REST API) ────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    tools: Option<Vec<GeminiTool>>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    role: String,   // "user" or "model"
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    FunctionCall { #[serde(rename = "functionCall")] function_call: GeminiFunctionCall },
    FunctionResponse { #[serde(rename = "functionResponse")] function_response: GeminiFunctionResponse },
}

#[derive(Serialize, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Serialize)]
struct GeminiTool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
}

// ── Message conversion ────────────────────────────────────────────────────────

fn to_gemini_contents(messages: &[Message]) -> Vec<GeminiContent> {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model",
                Role::System => "user", // Gemini doesn't have a system role
            }
            .to_string();

            let parts = msg
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(GeminiPart::Text { text: text.clone() }),
                    ContentBlock::ToolUse { name, input, .. } => {
                        Some(GeminiPart::FunctionCall {
                            function_call: GeminiFunctionCall {
                                name: name.clone(),
                                args: input.clone(),
                            },
                        })
                    }
                    ContentBlock::ToolResult { tool_use_id: _, content, .. } => {
                        Some(GeminiPart::FunctionResponse {
                            function_response: GeminiFunctionResponse {
                                name: "tool_result".to_string(),
                                response: serde_json::json!({ "content": content }),
                            },
                        })
                    }
                    _ => None,
                })
                .collect();

            GeminiContent { role, parts }
        })
        .collect()
}

fn from_gemini_response(resp: GeminiResponse, model: &str) -> Result<ModelResponse> {
    let candidate = resp
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| ClidoError::Provider("Gemini returned no candidates".to_string()))?;

    let stop_reason = match candidate.finish_reason.as_deref() {
        Some("STOP") => StopReason::EndTurn,
        Some("MAX_TOKENS") => StopReason::MaxTokens,
        _ => StopReason::EndTurn,
    };

    let content = candidate
        .content
        .parts
        .into_iter()
        .filter_map(|part| match part {
            GeminiPart::Text { text } => Some(ContentBlock::Text { text }),
            GeminiPart::FunctionCall { function_call } => Some(ContentBlock::ToolUse {
                id: uuid::Uuid::new_v4().to_string(),
                name: function_call.name,
                input: function_call.args,
            }),
            _ => None,
        })
        .collect();

    // Check for tool use in content to override stop reason
    let has_tool_use = content
        .iter()
        .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

    let usage = resp.usage_metadata.as_ref().map(|u| Usage {
        input_tokens: u.prompt_token_count.unwrap_or(0),
        output_tokens: u.candidates_token_count.unwrap_or(0),
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    }).unwrap_or_default();

    Ok(ModelResponse {
        content,
        stop_reason: if has_tool_use { StopReason::ToolUse } else { stop_reason },
        usage,
        model: model.to_string(),
    })
}

// ── ModelProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl ModelProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &AgentConfig,
    ) -> Result<ModelResponse> {
        let model = &config.model;
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let gemini_tools = if tools.is_empty() {
            None
        } else {
            Some(vec![GeminiTool {
                function_declarations: tools
                    .iter()
                    .map(|t| GeminiFunctionDeclaration {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.input_schema.clone(),
                    })
                    .collect(),
            }])
        };

        let request = GeminiRequest {
            contents: to_gemini_contents(messages),
            tools: gemini_tools,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: 8192,
            },
        };

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ClidoError::Provider(format!("Gemini HTTP error: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClidoError::Provider(format!(
                "Gemini API error: {}",
                body
            )));
        }

        let gemini_resp: GeminiResponse = resp
            .json()
            .await
            .map_err(|e| ClidoError::Provider(format!("Gemini parse error: {}", e)))?;

        from_gemini_response(gemini_resp, model)
    }
}
```

### Step 2: Export from the crate

In `crates/clido-providers/src/lib.rs`:

```rust
mod gemini;
pub use gemini::GeminiProvider;
```

### Step 3: Add to `make_provider()`

In `crates/clido-providers/src/lib.rs` (or wherever `make_provider()` lives), add the new branch:

```rust
pub fn make_provider(config: &ProviderConfig) -> Result<Box<dyn ModelProvider>> {
    match config.provider_type {
        ProviderType::Anthropic => Ok(Box::new(AnthropicProvider::new(...))),
        ProviderType::OpenAI    => Ok(Box::new(OpenAIProvider::new(...))),
        ProviderType::OpenRouter => Ok(Box::new(OpenRouterProvider::new(...))),
        ProviderType::Local     => Ok(Box::new(LocalProvider::new(...))),
        // Add:
        ProviderType::Gemini    => {
            let api_key = resolve_api_key(config, "GEMINI_API_KEY")?;
            Ok(Box::new(GeminiProvider::new(api_key)))
        }
    }
}
```

### Step 4: Add `Gemini` to `ProviderType`

In `crates/clido-core/src/config.rs`:

```rust
pub enum ProviderType {
    Anthropic,
    OpenAI,
    OpenRouter,
    Alibaba,
    Local,
    Gemini,   // add this
}
```

Update `validate_provider()` in `config_loader.rs` to include `"gemini"`.

### Step 5: Add pricing data

Add the Gemini models to the pricing JSON file so `clido list-models` and cost tracking work correctly.

### Step 6: Write tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_messages_to_gemini_format() {
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text: "Hello".to_string() }],
        }];
        let result = to_gemini_contents(&messages);
        assert_eq!(result[0].role, "user");
    }
}
```

## Error mapping

Map provider-specific errors to `ClidoError::Provider(String)`. The agent loop handles this error type for retries and user-visible messages.

Common patterns:
- HTTP 401/403 → mention the API key
- HTTP 429 → rate limit; suggest waiting
- HTTP 5xx → transient error; the agent loop can retry

## Token counting

The `Usage` struct includes `input_tokens` and `output_tokens`. These are used for:
- Cost calculation (via `clido-core::pricing::compute_cost_usd()`)
- Context window management (via `clido-context`)
- Session stats and the TUI status strip

Always populate these accurately. If the provider does not return token counts, estimate them using `clido-context::estimate_tokens_str()`.
