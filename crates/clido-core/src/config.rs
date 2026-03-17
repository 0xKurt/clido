//! Agent and provider configuration types (from config.toml / CLI).

use serde::{Deserialize, Serialize};

/// Permission mode for state-changing tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptAll,
    PlanOnly,
}

/// Agent-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_turns: u32,
    pub max_budget_usd: Option<f64>,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub permission_mode: PermissionMode,
    #[serde(default)]
    pub use_planner: bool,
    #[serde(default)]
    pub use_index: bool,
    /// Max context tokens (from config or pricing). None = use default in context engine (e.g. 200000).
    #[serde(default)]
    pub max_context_tokens: Option<u32>,
    /// Compact when context_tokens > max_context_tokens * compaction_threshold. Default 0.75.
    #[serde(default)]
    pub compaction_threshold: Option<f64>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            max_budget_usd: Some(5.0),
            model: String::new(),
            system_prompt: None,
            permission_mode: PermissionMode::Default,
            use_planner: false,
            use_index: false,
            max_context_tokens: None,
            compaction_threshold: None,
        }
    }
}

/// Provider type (canonical names from config spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    OpenAI,
    OpenRouter,
    Alibaba,
    Local,
}

/// Provider-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_from_json() {
        let json = r#"{
            "max_turns": 20,
            "max_budget_usd": 1.0,
            "model": "claude-3-5-sonnet",
            "permission_mode": "plan-only",
            "use_planner": false,
            "use_index": false
        }"#;
        let c: AgentConfig = serde_json::from_str(json).unwrap();
        assert_eq!(c.max_turns, 20);
        assert_eq!(c.max_budget_usd, Some(1.0));
        assert_eq!(c.model, "claude-3-5-sonnet");
        assert_eq!(c.permission_mode, PermissionMode::PlanOnly);
    }
}
