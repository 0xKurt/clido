//! Edit tool: replace old_string with new_string in file.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::path_guard::PathGuard;
use crate::secrets::scan_for_secrets;
use crate::{Tool, ToolOutput};

pub struct EditTool {
    guard: PathGuard,
}

impl EditTool {
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
impl Tool for EditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        "Replace old_string with new_string in file. Use replace_all for multiple occurrences."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" },
                "path": { "type": "string" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" },
                "replace_all": { "type": "boolean", "default": false }
            },
            "required": ["old_string"]
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let path_str = input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let old_string = input
            .get("old_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new_string = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if path_str.is_empty() {
            return ToolOutput::err("Missing required field: file_path or path".to_string());
        }
        if old_string.is_empty() {
            return ToolOutput::err("Missing required field: old_string".to_string());
        }

        let path = match self.guard.resolve_and_check(path_str) {
            Ok(p) => p,
            Err(e) => return ToolOutput::err(e),
        };

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => return ToolOutput::err(e.to_string()),
        };

        // Secret detection: warn on new_string content, but do not block
        let findings = scan_for_secrets(new_string);
        for finding in &findings {
            eprintln!(
                "Warning: potential secret detected in edit content: {}",
                finding
            );
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            if let Some(pos) = content.find(old_string) {
                let mut out = content;
                out.replace_range(pos..pos + old_string.len(), new_string);
                out
            } else {
                return ToolOutput::err(format!(
                    "<tool_use_error>String to replace not found in file.\nString: {}</tool_use_error>",
                    old_string
                ));
            }
        };

        if let Err(e) = tokio::fs::write(&path, &new_content).await {
            return ToolOutput::err(e.to_string());
        }

        let hash = hex::encode(Sha256::digest(new_content.as_bytes()));
        let mtime_nanos = tokio::fs::metadata(&path)
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        ToolOutput::ok_with_meta(
            format!("The file {} has been updated successfully.", path.display()),
            path.display().to_string(),
            hash,
            mtime_nanos,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn edit_basic_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "hello world").unwrap();
        let tool = EditTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({
                "file_path": "f.txt",
                "old_string": "world",
                "new_string": "rust"
            }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello rust");
    }

    #[tokio::test]
    async fn edit_replace_all() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "a a a").unwrap();
        let tool = EditTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({
                "file_path": "f.txt",
                "old_string": "a",
                "new_string": "b",
                "replace_all": true
            }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "b b b");
    }

    #[tokio::test]
    async fn edit_string_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "hello").unwrap();
        let tool = EditTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({
                "file_path": "f.txt",
                "old_string": "not_there",
                "new_string": "x"
            }))
            .await;
        assert!(out.is_error);
        assert!(out.content.contains("not found"));
    }

    #[tokio::test]
    async fn edit_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "old_string": "x", "new_string": "y" }))
            .await;
        assert!(out.is_error);
        assert!(out.content.contains("Missing"));
    }

    #[tokio::test]
    async fn edit_missing_old_string() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "file_path": "f.txt", "new_string": "y" }))
            .await;
        assert!(out.is_error);
    }

    #[tokio::test]
    async fn edit_path_alias() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g.txt");
        std::fs::write(&path, "foo bar").unwrap();
        let tool = EditTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({
                "path": "g.txt",
                "old_string": "foo",
                "new_string": "baz"
            }))
            .await;
        assert!(!out.is_error, "error: {}", out.content);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "baz bar");
    }
}
