# Core type definitions

This document defines the concrete Rust struct and enum types used across Clido, with field names, types, and serialization expectations. JSON wire examples are included for provider and session interchange.

---

## Message and content blocks

### Message

```rust
/// A single message in the conversation history (API and session).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "user" | "assistant" | "system"
    pub role: String,
    /// Content blocks (text, tool_use, tool_result, image).
    pub content: Vec<ContentBlock>,
}
```

**JSON example (user message):**

```json
{
  "role": "user",
  "content": [
    { "type": "text", "text": "List the files in src/" }
  ]
}
```

**JSON example (assistant with tool use):**

```json
{
  "role": "assistant",
  "content": [
    { "type": "text", "text": "I'll list the directory." },
    {
      "type": "tool_use",
      "id": "toolu_01A",
      "name": "Bash",
      "input": { "command": "ls -la src/" }
    }
  ]
}
```

---

### ContentBlock

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    Image {
        source: ImageSource,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    pub media_type: String,
    pub data: String,
}
```

**JSON examples:**

- Text: `{ "type": "text", "text": "Hello." }`
- ToolUse: `{ "type": "tool_use", "id": "toolu_01A", "name": "Read", "input": { "path": "src/main.rs" } }`
- ToolResult: `{ "type": "tool_result", "tool_use_id": "toolu_01A", "content": "fn main() { ... }", "is_error": false }`

---

### ToolUse (standalone, for executor)

```rust
#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}
```

---

### ToolResult (standalone, for executor)

```rust
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}
```

---

## Model response and usage

### ModelResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
}
```

**JSON example:**

```json
{
  "content": [
    { "type": "text", "text": "Here are the files:\n\nsrc/main.rs\nsrc/lib.rs" }
  ],
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 1500,
    "output_tokens": 45,
    "cache_creation_input_tokens": null,
    "cache_read_input_tokens": 800
  }
}
```

---

### Usage

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
}
```

**JSON example:** See `ModelResponse` above.

---

## Errors

### ClidoError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClidoError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Budget exceeded: {0}")]
    Budget(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Planner error: {0}")]
    Planner(String),
}
```

User-facing messages and hints are attached to each variant; see CLI spec Section 6 for message templates.

---

## ToolOutput (tool execution result)

```rust
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}
```

This is the in-memory result of a tool execution, not the wire format. Wire format for tool results in messages uses `ContentBlock::ToolResult`.
