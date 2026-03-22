//! Message, content block, and API response types aligned with session and CLI spec.

use serde::{Deserialize, Serialize};

/// Chat role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Content block (mirrors Anthropic API).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
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
    Thinking {
        thinking: String,
    },
    /// Base64-encoded image block (sent to vision-capable models).
    Image {
        media_type: String,
        base64_data: String,
    },
}

/// Single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

/// Token usage from the model response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

/// Why the model stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

/// Tool schema for API (name, description, JSON Schema for input).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Full model API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_json_roundtrip() {
        for role in [Role::User, Role::Assistant, Role::System] {
            let j = serde_json::to_string(&role).unwrap();
            let r: Role = serde_json::from_str(&j).unwrap();
            assert_eq!(role, r);
        }
    }

    #[test]
    fn content_block_text_roundtrip() {
        let b = ContentBlock::Text {
            text: "hello".to_string(),
        };
        let j = serde_json::to_string(&b).unwrap();
        let b2: ContentBlock = serde_json::from_str(&j).unwrap();
        match (&b, &b2) {
            (ContentBlock::Text { text: t1 }, ContentBlock::Text { text: t2 }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("mismatch"),
        }
    }

    #[test]
    fn content_block_tool_use_roundtrip() {
        let b = ContentBlock::ToolUse {
            id: "toolu_01".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({"path": "src/main.rs"}),
        };
        let j = serde_json::to_string(&b).unwrap();
        let b2: ContentBlock = serde_json::from_str(&j).unwrap();
        match (&b2, &b) {
            (
                ContentBlock::ToolUse { id, name, input },
                ContentBlock::ToolUse {
                    id: i2,
                    name: n2,
                    input: i2v,
                },
            ) => {
                assert_eq!(id, i2);
                assert_eq!(name, n2);
                assert_eq!(input, i2v);
            }
            _ => panic!("mismatch"),
        }
    }

    #[test]
    fn message_roundtrip() {
        let m = Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "hi".to_string(),
            }],
        };
        let j = serde_json::to_string(&m).unwrap();
        let m2: Message = serde_json::from_str(&j).unwrap();
        assert_eq!(m.role, m2.role);
        assert_eq!(m.content.len(), m2.content.len());
    }

    #[test]
    fn usage_roundtrip() {
        let u = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: Some(10),
            cache_read_input_tokens: Some(80),
        };
        let j = serde_json::to_string(&u).unwrap();
        let u2: Usage = serde_json::from_str(&j).unwrap();
        assert_eq!(u.input_tokens, u2.input_tokens);
        assert_eq!(u.output_tokens, u2.output_tokens);
    }

    #[test]
    fn model_response_roundtrip() {
        let r = ModelResponse {
            id: "msg_1".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            content: vec![ContentBlock::Text {
                text: "Done.".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        };
        let j = serde_json::to_string(&r).unwrap();
        let r2: ModelResponse = serde_json::from_str(&j).unwrap();
        assert_eq!(r.id, r2.id);
        assert_eq!(r.stop_reason, r2.stop_reason);
    }
}
