//! Glob tool: list files matching pattern.

use async_trait::async_trait;
use glob::Pattern;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use crate::path_guard::PathGuard;
use crate::{Tool, ToolOutput};

pub struct GlobTool {
    guard: PathGuard,
}

impl GlobTool {
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
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "List files matching a glob pattern. Pattern and optional path (default cwd)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern (e.g. **/*.rs)" },
                "path": { "type": "string", "description": "Directory to search (default: cwd)" }
            },
            "required": ["pattern"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let pattern = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let path_str = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        if pattern.is_empty() {
            return ToolOutput::err("Missing required field: pattern".to_string());
        }

        let base = if path_str == "." || path_str.is_empty() {
            self.guard.workspace_root().to_path_buf()
        } else {
            match self.guard.resolve_and_check(path_str) {
                Ok(p) => p,
                Err(e) => return ToolOutput::err(e),
            }
        };

        if !base.is_dir() {
            return ToolOutput::err(format!("Path is not a directory: {}", base.display()));
        }

        let pattern = match Pattern::new(pattern) {
            Ok(p) => p,
            Err(e) => return ToolOutput::err(e.to_string()),
        };

        let mut entries: Vec<PathBuf> = Vec::new();
        for result in WalkBuilder::new(&base).build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() && !self.guard.is_blocked(path) {
                        entries.push(path.to_path_buf());
                    }
                }
                Err(e) => return ToolOutput::err(e.to_string()),
            }
        }

        let mut matched: Vec<String> = entries
            .into_iter()
            .filter_map(|p| {
                let rel = p.strip_prefix(&base).ok()?;
                let s = rel.to_string_lossy();
                if pattern.matches_path(Path::new(&*s)) {
                    Some(s.into_owned())
                } else {
                    None
                }
            })
            .collect();
        matched.sort();

        ToolOutput::ok(matched.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn glob_finds_rs_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();
        std::fs::write(dir.path().join("config.toml"), "[package]").unwrap();
        let tool = GlobTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({ "pattern": "*.rs" })).await;
        assert!(!out.is_error, "error: {}", out.content);
        assert!(out.content.contains("main.rs"), "content: {}", out.content);
        assert!(out.content.contains("lib.rs"), "content: {}", out.content);
        assert!(!out.content.contains("config.toml"));
    }

    #[tokio::test]
    async fn glob_missing_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GlobTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(out.is_error);
        assert!(out.content.contains("pattern"));
    }

    #[tokio::test]
    async fn glob_no_matches_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "").unwrap();
        let tool = GlobTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({ "pattern": "*.py" })).await;
        assert!(!out.is_error);
        assert!(out.content.is_empty());
    }

    /// Lines 47-48: is_read_only returns true.
    #[test]
    fn glob_tool_is_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GlobTool::new(dir.path().to_path_buf());
        assert!(tool.is_read_only());
    }

    /// Line 53: path_str is provided explicitly (non-default).
    #[tokio::test]
    async fn glob_with_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
        let tool = GlobTool::new(dir.path().to_path_buf());
        let canonical_sub = std::fs::canonicalize(&sub).unwrap();
        let out = tool
            .execute(serde_json::json!({
                "pattern": "*.rs",
                "path": canonical_sub.to_str().unwrap()
            }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        assert!(out.content.contains("main.rs"), "content: {}", out.content);
    }

    #[tokio::test]
    async fn glob_nested_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("lib.rs"), "").unwrap();
        let tool = GlobTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "pattern": "**/*.rs" }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        assert!(out.content.contains("lib.rs"), "content: {}", out.content);
    }
}
