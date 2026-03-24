//! SpawnWorkerTool and SpawnReviewerTool: invoke sub-agents for mechanical subtasks and review.
//!
//! Both tools live in `clido-cli` rather than `clido-tools` to avoid a circular dependency:
//! `clido-agent` depends on `clido-tools`, so `clido-tools` cannot import `clido-agent`.

use async_trait::async_trait;
use clido_core::AgentConfig;
use clido_providers::ModelProvider;
use clido_tools::{default_registry, Tool, ToolOutput};
use std::sync::Arc;

/// Spawn a worker sub-agent for mechanical subtasks (filtering, summarizing, extracting, formatting).
/// The worker runs with a narrow context slice and returns structured output.
pub struct SpawnWorkerTool {
    provider: Arc<dyn ModelProvider>,
    config: AgentConfig,
    workspace: std::path::PathBuf,
}

impl SpawnWorkerTool {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        config: AgentConfig,
        workspace: std::path::PathBuf,
    ) -> Self {
        Self {
            provider,
            config,
            workspace,
        }
    }
}

#[async_trait]
impl Tool for SpawnWorkerTool {
    fn name(&self) -> &str {
        "SpawnWorker"
    }

    fn description(&self) -> &str {
        "Spawn a worker sub-agent for a mechanical subtask (file filtering, summarizing, \
         extracting structured data, formatting). Pass only the minimal context needed — \
         never the full conversation. The worker returns its result as text."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Clear, focused task description for the worker agent."
                },
                "context": {
                    "type": "string",
                    "description": "Minimal context the worker needs (file content, list, etc.). Do not include the full conversation."
                },
                "output_format": {
                    "type": "string",
                    "description": "Expected output format hint, e.g. 'JSON array of filenames' or 'plain text summary'.",
                    "default": "plain text"
                }
            },
            "required": ["task", "context"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let task = match input["task"].as_str() {
            Some(t) => t.to_string(),
            None => return ToolOutput::err("SpawnWorker: missing 'task' field".into()),
        };
        let context = input["context"].as_str().unwrap_or("").to_string();
        let output_format = input["output_format"].as_str().unwrap_or("plain text");

        let prompt = format!(
            "You are a focused worker agent. Complete this task and return only the result.\n\
             Output format: {}\n\n\
             Context:\n{}\n\n\
             Task: {}",
            output_format, context, task
        );

        let worker_registry = default_registry(self.workspace.clone());
        let mut sub =
            clido_agent::SubAgent::new(self.provider.clone(), worker_registry, self.config.clone());

        match sub.run(&prompt).await {
            Ok(result) => ToolOutput::ok(result),
            Err(e) => ToolOutput::err(format!("Worker sub-agent failed: {}", e)),
        }
    }
}

/// Spawn a reviewer sub-agent to check the quality of a completed task output.
pub struct SpawnReviewerTool {
    provider: Arc<dyn ModelProvider>,
    config: AgentConfig,
    workspace: std::path::PathBuf,
}

impl SpawnReviewerTool {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        config: AgentConfig,
        workspace: std::path::PathBuf,
    ) -> Self {
        Self {
            provider,
            config,
            workspace,
        }
    }
}

#[async_trait]
impl Tool for SpawnReviewerTool {
    fn name(&self) -> &str {
        "SpawnReviewer"
    }

    fn description(&self) -> &str {
        "Spawn a reviewer sub-agent to perform a final quality check on the completed output. \
         Use this when the main task is done and a second-opinion review is warranted. \
         The reviewer returns feedback or a pass/fail verdict."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "output": {
                    "type": "string",
                    "description": "The completed output to review."
                },
                "criteria": {
                    "type": "string",
                    "description": "What the reviewer should check for (correctness, completeness, style, etc.)."
                }
            },
            "required": ["output", "criteria"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let output = match input["output"].as_str() {
            Some(o) => o.to_string(),
            None => return ToolOutput::err("SpawnReviewer: missing 'output' field".into()),
        };
        let criteria = input["criteria"]
            .as_str()
            .unwrap_or("correctness and completeness");

        let prompt = format!(
            "You are a reviewer agent. Evaluate the following output and return a concise review.\n\
             Criteria: {}\n\n\
             Output to review:\n{}",
            criteria, output
        );

        let reviewer_registry = default_registry(self.workspace.clone());
        let mut sub = clido_agent::SubAgent::new(
            self.provider.clone(),
            reviewer_registry,
            self.config.clone(),
        );

        match sub.run(&prompt).await {
            Ok(result) => ToolOutput::ok(result),
            Err(e) => ToolOutput::err(format!("Reviewer sub-agent failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clido_core::AgentConfig;

    fn dummy_config() -> AgentConfig {
        AgentConfig {
            model: "test-model".to_string(),
            max_turns: 1,
            ..Default::default()
        }
    }

    // ── SpawnWorkerTool metadata ──────────────────────────────────────────

    #[test]
    fn spawn_worker_name() {
        let t = SpawnWorkerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        assert_eq!(t.name(), "SpawnWorker");
    }

    #[test]
    fn spawn_worker_is_read_only() {
        let t = SpawnWorkerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        assert!(t.is_read_only());
    }

    #[test]
    fn spawn_worker_schema_has_required_fields() {
        let t = SpawnWorkerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        let schema = t.schema();
        let required = schema["required"]
            .as_array()
            .expect("required must be array");
        let req_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(req_names.contains(&"task"), "schema must require 'task'");
        assert!(
            req_names.contains(&"context"),
            "schema must require 'context'"
        );
        // output_format is optional
        assert!(
            !req_names.contains(&"output_format"),
            "output_format should be optional"
        );
    }

    #[test]
    fn spawn_worker_description_not_empty() {
        let t = SpawnWorkerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        assert!(!t.description().is_empty());
    }

    // ── SpawnReviewerTool metadata ────────────────────────────────────────

    #[test]
    fn spawn_reviewer_name() {
        let t = SpawnReviewerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        assert_eq!(t.name(), "SpawnReviewer");
    }

    #[test]
    fn spawn_reviewer_is_read_only() {
        let t = SpawnReviewerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        assert!(t.is_read_only());
    }

    #[test]
    fn spawn_reviewer_schema_has_required_fields() {
        let t = SpawnReviewerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        let schema = t.schema();
        let required = schema["required"]
            .as_array()
            .expect("required must be array");
        let req_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            req_names.contains(&"output"),
            "schema must require 'output'"
        );
        assert!(
            req_names.contains(&"criteria"),
            "schema must require 'criteria'"
        );
    }

    #[test]
    fn spawn_reviewer_description_not_empty() {
        let t = SpawnReviewerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        assert!(!t.description().is_empty());
    }

    // ── execute: missing required fields returns error ────────────────────

    #[tokio::test]
    async fn spawn_worker_missing_task_returns_error() {
        let t = SpawnWorkerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        let result = t
            .execute(serde_json::json!({ "context": "some ctx" }))
            .await;
        assert!(result.is_error, "missing task should return error");
        assert!(result.content.contains("missing 'task'"));
    }

    #[tokio::test]
    async fn spawn_reviewer_missing_output_returns_error() {
        let t = SpawnReviewerTool::new(
            Arc::new(NullProvider),
            dummy_config(),
            std::path::PathBuf::from("."),
        );
        let result = t
            .execute(serde_json::json!({ "criteria": "correctness" }))
            .await;
        assert!(result.is_error, "missing output should return error");
        assert!(result.content.contains("missing 'output'"));
    }

    // ── Minimal provider stub ─────────────────────────────────────────────

    struct NullProvider;

    #[async_trait]
    impl clido_providers::ModelProvider for NullProvider {
        async fn complete(
            &self,
            _messages: &[clido_core::Message],
            _tools: &[clido_core::ToolSchema],
            _config: &clido_core::AgentConfig,
        ) -> clido_core::Result<clido_core::ModelResponse> {
            use clido_core::{ModelResponse, StopReason, Usage};
            Ok(ModelResponse {
                id: "null".to_string(),
                content: vec![],
                stop_reason: StopReason::EndTurn,
                usage: Usage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                },
                model: "null".to_string(),
            })
        }

        async fn complete_stream(
            &self,
            _messages: &[clido_core::Message],
            _tools: &[clido_core::ToolSchema],
            _config: &clido_core::AgentConfig,
        ) -> clido_core::Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<Item = clido_core::Result<clido_providers::StreamEvent>>
                        + Send,
                >,
            >,
        > {
            Ok(Box::pin(futures::stream::empty()))
        }

        async fn list_models(&self) -> Vec<clido_providers::ModelEntry> {
            vec![]
        }
    }
}
