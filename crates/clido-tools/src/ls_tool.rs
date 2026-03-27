//! Ls tool: list the contents of a directory.

use async_trait::async_trait;
use std::path::PathBuf;

use crate::path_guard::PathGuard;
use crate::{Tool, ToolOutput};

/// Maximum depth allowed for recursive listing.
const MAX_DEPTH: u32 = 10;
/// Maximum number of entries to return (prevents token overflow on huge directories).
const MAX_ENTRIES: usize = 500;

pub struct LsTool {
    guard: PathGuard,
}

impl LsTool {
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
impl Tool for LsTool {
    fn name(&self) -> &str {
        "Ls"
    }

    fn description(&self) -> &str {
        "List the contents of a directory. Returns files and subdirectory names. \
        Use `path` to specify the target directory (default: workspace root). \
        Use `depth` to recurse (default 1, max 10). Hidden files (dotfiles) are \
        excluded unless `show_hidden` is true."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to list. Defaults to the workspace root."
                },
                "depth": {
                    "type": "integer",
                    "description": "How many directory levels to recurse. 1 = immediate children only (default).",
                    "default": 1,
                    "minimum": 1,
                    "maximum": 10
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Include hidden (dot) files and directories. Default false.",
                    "default": false
                }
            }
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let path_str = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let depth = input
            .get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .min(MAX_DEPTH as u64) as u32;
        let show_hidden = input
            .get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let base = if path_str == "." || path_str.is_empty() {
            self.guard.workspace_root().to_path_buf()
        } else {
            match self.guard.resolve_and_check(path_str) {
                Ok(p) => p,
                Err(e) => return ToolOutput::err(e),
            }
        };

        if !base.exists() {
            return ToolOutput::err(format!("Path does not exist: {}", base.display()));
        }
        if !base.is_dir() {
            return ToolOutput::err(format!("Not a directory: {}", base.display()));
        }

        let mut lines: Vec<String> = Vec::new();
        collect_entries(&base, &base, 0, depth, show_hidden, &self.guard, &mut lines);

        if lines.is_empty() {
            return ToolOutput::ok("(empty directory)".to_string());
        }

        let truncated = lines.len() > MAX_ENTRIES;
        lines.truncate(MAX_ENTRIES);
        if truncated {
            lines.push(format!("... (output truncated at {} entries)", MAX_ENTRIES));
        }

        ToolOutput::ok(lines.join("\n"))
    }
}

/// Recursively collect directory entries into `out`, stopping at `max_depth`.
fn collect_entries(
    base: &std::path::Path,
    dir: &std::path::Path,
    current_depth: u32,
    max_depth: u32,
    show_hidden: bool,
    guard: &PathGuard,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_ENTRIES {
        return;
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut entries: Vec<std::fs::DirEntry> = read_dir
        .filter_map(|r| r.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            if !show_hidden && name_str.starts_with('.') {
                return false;
            }
            !guard.is_blocked(&e.path())
        })
        .collect();

    // Sort: directories first, then files, both alphabetically.
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        if out.len() >= MAX_ENTRIES {
            return;
        }
        let path = entry.path();
        let rel = match path.strip_prefix(base) {
            Ok(r) => r,
            Err(_) => &path,
        };
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let display = if is_dir {
            format!("{}/", rel.to_string_lossy())
        } else {
            rel.to_string_lossy().into_owned()
        };
        out.push(display);

        if is_dir && current_depth + 1 < max_depth {
            collect_entries(
                base,
                &path,
                current_depth + 1,
                max_depth,
                show_hidden,
                guard,
                out,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ls_lists_immediate_children() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.txt"), "hello").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(!out.is_error, "error: {}", out.content);
        assert!(
            out.content.contains("subdir/"),
            "missing subdir: {}",
            out.content
        );
        assert!(
            out.content.contains("file.txt"),
            "missing file: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn ls_depth_1_does_not_recurse() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), "").unwrap();

        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({ "depth": 1 })).await;
        assert!(!out.is_error);
        assert!(
            !out.content.contains("nested.txt"),
            "should not recurse: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn ls_depth_2_recurses_one_level() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), "").unwrap();

        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({ "depth": 2 })).await;
        assert!(!out.is_error);
        assert!(
            out.content.contains("nested.txt"),
            "should recurse: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn ls_hides_dotfiles_by_default() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        std::fs::write(dir.path().join("visible.txt"), "").unwrap();

        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(!out.is_error);
        assert!(
            !out.content.contains(".hidden"),
            "hidden should be excluded: {}",
            out.content
        );
        assert!(out.content.contains("visible.txt"));
    }

    #[tokio::test]
    async fn ls_show_hidden_includes_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();

        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "show_hidden": true }))
            .await;
        assert!(!out.is_error);
        assert!(
            out.content.contains(".hidden"),
            "hidden should be shown: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn ls_nonexistent_path_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({ "path": "/nonexistent/path/xyz" }))
            .await;
        assert!(out.is_error);
    }

    #[tokio::test]
    async fn ls_empty_directory_returns_message() {
        let dir = tempfile::tempdir().unwrap();
        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(!out.is_error);
        assert!(
            out.content.contains("empty"),
            "should say empty: {}",
            out.content
        );
    }

    #[test]
    fn ls_is_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let tool = LsTool::new(dir.path().to_path_buf());
        assert!(tool.is_read_only());
    }

    #[tokio::test]
    async fn ls_dirs_sorted_before_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a_file.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("a_dir")).unwrap();

        let tool = LsTool::new(dir.path().to_path_buf());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(!out.is_error);
        let lines: Vec<&str> = out.content.lines().collect();
        let dir_pos = lines.iter().position(|l| l.ends_with('/')).unwrap();
        let file_pos = lines.iter().position(|l| l.ends_with(".txt")).unwrap();
        assert!(dir_pos < file_pos, "dirs should come before files");
    }
}
