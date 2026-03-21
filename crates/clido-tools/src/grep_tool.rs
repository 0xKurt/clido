//! Grep tool: search for pattern in files.

use async_trait::async_trait;
use ignore::WalkBuilder;
use regex::Regex;
use std::path::PathBuf;

use crate::path_guard::PathGuard;
use crate::{Tool, ToolOutput};

pub struct GrepTool {
    guard: PathGuard,
}

impl GrepTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            guard: PathGuard::new(workspace_root),
        }
    }
    pub fn new_with_guard(guard: PathGuard) -> Self {
        Self { guard }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        "Search for pattern in files. Supports output_mode: content, files_with_matches, count."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string" },
                "output_mode": { "type": "string", "enum": ["content", "files_with_matches", "count"] },
                "context": { "type": "integer" },
                "i": { "type": "boolean" },
                "head_limit": { "type": "integer" }
            },
            "required": ["pattern"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let known_keys = [
            "pattern",
            "path",
            "output_mode",
            "context",
            "i",
            "head_limit",
        ];
        if let Some(obj) = input.as_object() {
            for (k, _) in obj {
                if !known_keys.contains(&k.as_str()) {
                    return ToolOutput::err(format!(
                        "<tool_use_error>InputValidationError: Grep failed due to the following issue:\nAn unexpected parameter `{}` was provided</tool_use_error>",
                        k
                    ));
                }
            }
        }

        let pattern = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        if pattern.is_empty() {
            return ToolOutput::err("Missing required field: pattern".to_string());
        }

        let path_str = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let base = if path_str == "." || path_str.is_empty() {
            self.guard.workspace_root().to_path_buf()
        } else {
            match self.guard.resolve_and_check(path_str) {
                Ok(p) => p,
                Err(e) => return ToolOutput::err(e),
            }
        };

        let output_mode = input
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");
        let case_insensitive = input.get("i").and_then(|v| v.as_bool()).unwrap_or(false);
        let head_limit = input
            .get("head_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let re = if case_insensitive {
            Regex::new(&format!("(?i){}", pattern))
        } else {
            Regex::new(pattern)
        };
        let re = match re {
            Ok(r) => r,
            Err(e) => {
                return ToolOutput::err(e.to_string());
            }
        };

        let mut file_matches: Vec<String> = Vec::new();
        let mut content_lines: Vec<String> = Vec::new();
        let mut total_count = 0u64;

        for result in WalkBuilder::new(&base).build() {
            let entry: ignore::DirEntry = match result {
                Ok(e) => e,
                Err(e) => return ToolOutput::err(e.to_string()),
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if self.guard.is_blocked(path) {
                continue;
            }
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let rel_path = path
                .strip_prefix(self.guard.workspace_root())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.display().to_string());
            let mut file_count = 0u64;
            for (line_no, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    total_count += 1;
                    file_count += 1;
                    if output_mode == "content" {
                        content_lines.push(format!("{}:{}:{}", rel_path, line_no + 1, line.trim()));
                    }
                }
            }
            if output_mode == "files_with_matches" && file_count > 0 {
                let rel = path
                    .strip_prefix(self.guard.workspace_root())
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| path.display().to_string());
                file_matches.push(rel);
            }
        }

        let content = match output_mode {
            "count" => total_count.to_string(),
            "files_with_matches" => {
                file_matches.sort();
                file_matches.join("\n")
            }
            _ => {
                let mut m = content_lines;
                if head_limit > 0 {
                    m.truncate(head_limit);
                }
                m.join("\n")
            }
        };

        ToolOutput::ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "foo bar\nbaz qux\nfoo again").unwrap();
        std::fs::write(dir.path().join("b.txt"), "hello world").unwrap();
        dir
    }

    #[tokio::test]
    async fn grep_files_with_matches() {
        let dir = setup_dir();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({ "pattern": "foo" })).await;
        assert!(!out.is_error, "error: {}", out.content);
        assert!(out.content.contains("a.txt"), "content: {}", out.content);
        assert!(!out.content.contains("b.txt"));
    }

    #[tokio::test]
    async fn grep_content_mode() {
        let dir = setup_dir();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "pattern": "foo", "output_mode": "content" }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        assert!(out.content.contains("foo"), "content: {}", out.content);
    }

    #[tokio::test]
    async fn grep_count_mode() {
        let dir = setup_dir();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "pattern": "foo", "output_mode": "count" }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        let count: u64 = out.content.trim().parse().unwrap();
        assert_eq!(count, 2); // "foo bar" and "foo again"
    }

    #[tokio::test]
    async fn grep_case_insensitive() {
        let dir = setup_dir();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "pattern": "FOO", "i": true, "output_mode": "count" }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        let count: u64 = out.content.trim().parse().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn grep_missing_pattern() {
        let dir = setup_dir();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(out.is_error);
    }

    #[tokio::test]
    async fn grep_unknown_param_rejected() {
        let dir = setup_dir();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "pattern": "x", "unknown_key": true }))
            .await;
        assert!(out.is_error);
        assert!(out.content.contains("unexpected parameter"));
    }

    #[tokio::test]
    async fn grep_head_limit() {
        let dir = tempfile::tempdir().unwrap();
        let content = (1..=10)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.path().join("f.txt"), &content).unwrap();
        let tool = GrepTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(
                serde_json::json!({ "pattern": "line", "output_mode": "content", "head_limit": 3 }),
            )
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        let lines: Vec<_> = out.content.lines().collect();
        assert_eq!(lines.len(), 3);
    }
}
