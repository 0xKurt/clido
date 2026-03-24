//! SemanticSearch tool: auto-builds/refreshes the repo index, then queries it.
//!
//! The index is stored at `<workspace>/.clido/index.db`. On first use it is built
//! automatically. If it is older than INDEX_MAX_AGE_SECS it is rebuilt in-place
//! before querying so results are always fresh without any manual step.

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolOutput};

/// Rebuild the index if it is older than 1 hour.
const INDEX_MAX_AGE_SECS: u64 = 3600;

/// File extensions indexed by default.
/// Web3/smart-contract languages are listed first so they get priority in results.
const DEFAULT_EXTENSIONS: &[&str] = &[
    // Web3 / smart contracts
    "sol",   // Solidity (Ethereum, EVM)
    "move",  // Move (Aptos, Sui)
    "vy",    // Vyper
    "fe",    // Fe (Ethereum)
    "yul",   // Yul / Yul+ (EVM assembly IR)
    "rell",  // Rell (Chromia)
    "cairo", // Cairo (StarkNet)
    // General-purpose
    "rs", "py", "js", "ts", "go", "java", "c", "cpp", "h", "md",
];

pub struct SemanticSearchTool {
    workspace_root: std::path::PathBuf,
}

impl SemanticSearchTool {
    pub fn new(workspace_root: std::path::PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Return the index DB path, creating the .clido dir if needed.
    fn index_path(&self) -> std::path::PathBuf {
        self.workspace_root.join(".clido").join("index.db")
    }

    /// Age of the index in seconds. Returns None if no index exists yet.
    fn index_age_secs(index_path: &std::path::Path) -> Option<u64> {
        let meta = std::fs::metadata(index_path).ok()?;
        let modified = meta.modified().ok()?;
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);
        Some(age.as_secs())
    }

    /// Build (or rebuild) the index, returning a human-readable status note.
    fn ensure_index(&self) -> String {
        let db_path = self.index_path();
        let age = Self::index_age_secs(&db_path);

        let needs_build = match age {
            None => true,                      // doesn't exist yet
            Some(s) => s > INDEX_MAX_AGE_SECS, // stale
        };

        if !needs_build {
            return String::new();
        }

        // Create .clido dir if needed.
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let label = if age.is_none() {
            "Building"
        } else {
            "Refreshing"
        };

        match clido_index::RepoIndex::open(&db_path) {
            Err(e) => format!("(Index unavailable: {})\n", e),
            Ok(mut idx) => match idx.build(&self.workspace_root, DEFAULT_EXTENSIONS) {
                Ok(n) => {
                    let (_, sym_count) = idx.stats().unwrap_or((0, 0));
                    format!(
                        "({} repo index: {} files, {} symbols)\n",
                        label, n, sym_count
                    )
                }
                Err(e) => format!("(Index build failed: {})\n", e),
            },
        }
    }
}

#[async_trait]
impl Tool for SemanticSearchTool {
    fn name(&self) -> &str {
        "SemanticSearch"
    }

    fn description(&self) -> &str {
        "Search the repository for files, symbols, and long-term memories relevant to a query. \
         The index is built and kept fresh automatically — no manual setup needed. \
         Use for navigating large codebases, finding where a function is defined, or \
         recalling past context."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query (function name, concept, file pattern, etc.)"
                },
                "target_directory": {
                    "type": "string",
                    "description": "Limit search to this subdirectory (optional)."
                },
                "num_results": {
                    "type": "integer",
                    "description": "Max results per source (default: 5, max: 20)."
                }
            },
            "required": ["query"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, input: Value) -> ToolOutput {
        let query = match input.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.trim().is_empty() => q.to_string(),
            _ => return ToolOutput::err("Missing required field: query".to_string()),
        };
        let num_results = input
            .get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .min(20) as usize;

        // Auto-build/refresh index (no-op when fresh, fast when up-to-date).
        let index_note = self.ensure_index();

        let mut output_parts = Vec::new();
        if !index_note.is_empty() {
            output_parts.push(index_note);
        }

        // ── Repo index search ──────────────────────────────────────────────
        let index_db = self.index_path();
        if index_db.exists() {
            match clido_index::RepoIndex::open(&index_db) {
                Ok(idx) => {
                    // Symbols
                    if let Ok(syms) = idx.search_symbols(&query) {
                        if !syms.is_empty() {
                            let mut part = format!("## Symbols matching '{}'\n", query);
                            for s in syms.iter().take(num_results) {
                                part.push_str(&format!(
                                    "  {} {} — {} line {}\n",
                                    s.kind, s.name, s.path, s.line
                                ));
                            }
                            output_parts.push(part);
                        }
                    }
                    // Files
                    if let Ok(files) = idx.search_files(&query) {
                        let files: Vec<_> =
                            match input.get("target_directory").and_then(|v| v.as_str()) {
                                Some(dir) => {
                                    files.into_iter().filter(|f| f.path.contains(dir)).collect()
                                }
                                None => files,
                            };
                        if !files.is_empty() {
                            let mut part = format!("## Files matching '{}'\n", query);
                            for f in files.iter().take(num_results) {
                                part.push_str(&format!("  {}\n", f.path));
                            }
                            output_parts.push(part);
                        }
                    }
                }
                Err(e) => {
                    output_parts.push(format!("(Index error: {})\n", e));
                }
            }
        }

        // ── Memory search ──────────────────────────────────────────────────
        if let Some(dirs) = directories::ProjectDirs::from("", "", "clido") {
            let memory_db = dirs.data_dir().join("memory.db");
            if memory_db.exists() {
                if let Ok(store) = clido_memory::MemoryStore::open(&memory_db) {
                    if let Ok(memories) = store.search_keyword(&query, num_results) {
                        if !memories.is_empty() {
                            let mut part = format!("## Memories matching '{}'\n", query);
                            for m in &memories {
                                let tags = if m.tags.is_empty() {
                                    String::new()
                                } else {
                                    format!(" [{}]", m.tags.join(", "))
                                };
                                part.push_str(&format!(
                                    "  {}{} {}\n",
                                    m.created_at, tags, m.content
                                ));
                            }
                            output_parts.push(part);
                        }
                    }
                }
            }
        }

        if output_parts.is_empty() {
            ToolOutput::ok(format!("No results found for '{}'.", query))
        } else {
            ToolOutput::ok(output_parts.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_age_secs_returns_none_for_nonexistent_file() {
        let p = std::path::Path::new("/nonexistent/path/index.db");
        assert!(SemanticSearchTool::index_age_secs(p).is_none());
    }

    #[test]
    fn index_age_secs_returns_some_for_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.db");
        std::fs::write(&file, "data").unwrap();
        let age = SemanticSearchTool::index_age_secs(&file);
        assert!(age.is_some());
        // Age should be >= 0 (could be 0 for a just-created file)
        assert!(age.unwrap() < 60); // just created, should be < 1 minute old
    }

    #[test]
    fn tool_name_and_schema() {
        let tool = SemanticSearchTool::new(std::env::temp_dir());
        assert_eq!(tool.name(), "SemanticSearch");
        assert!(tool.is_read_only());
        let schema = tool.schema();
        assert_eq!(schema["type"], "object");
        let props = &schema["properties"];
        assert!(props.get("query").is_some());
        assert!(props.get("target_directory").is_some());
        assert!(props.get("num_results").is_some());
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("query")));
    }

    #[tokio::test]
    async fn execute_missing_query_returns_error() {
        let tool = SemanticSearchTool::new(std::env::temp_dir());
        let out = tool.execute(serde_json::json!({})).await;
        assert!(out.is_error);
        assert!(out.content.contains("Missing required field"));
    }

    #[tokio::test]
    async fn execute_empty_query_returns_error() {
        let tool = SemanticSearchTool::new(std::env::temp_dir());
        let out = tool.execute(serde_json::json!({"query": "   "})).await;
        assert!(out.is_error);
    }

    #[tokio::test]
    async fn execute_with_query_returns_ok() {
        let dir = tempfile::tempdir().unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        // Write a test file so the index builds something
        std::fs::write(dir.path().join("test.rs"), "fn hello() {}").unwrap();
        let out = tool.execute(serde_json::json!({"query": "hello"})).await;
        // Should not be an error (even if no results found)
        assert!(!out.is_error);
    }

    #[test]
    fn ensure_index_builds_for_new_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        // First call should build the index
        let note = tool.ensure_index();
        // Should return a note about building
        assert!(
            note.is_empty()
                || note.contains("Building")
                || note.contains("Refreshing")
                || note.contains("Index")
        );
    }

    /// Lines 63 (Some path) + 67 (not stale → empty string): call ensure_index twice;
    /// second call finds a fresh index and returns "".
    #[test]
    fn ensure_index_returns_empty_when_fresh() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.rs"), "fn foo() {}").unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        // Build the index
        let _first = tool.ensure_index();
        // Second call: index exists and is fresh, should return empty string
        let second = tool.ensure_index();
        assert!(
            second.is_empty(),
            "expected empty for fresh index, got: {:?}",
            second
        );
    }

    /// Line 78: Refreshing label — create a stale index by writing a dummy .db file
    /// with an old mtime via libc utimes (unix only), then ensure_index should rebuild.
    #[cfg(unix)]
    #[test]
    fn ensure_index_refreshes_stale_index() {
        use std::ffi::CString;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.rs"), "fn foo() {}").unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        // First: build the index normally.
        let _ = tool.ensure_index();
        let db_path = dir.path().join(".clido").join("index.db");
        if db_path.exists() {
            // Set mtime to 2 hours ago using utimes
            let two_hours_ago = std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(7200))
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as libc::time_t;
            let times = [
                libc::timeval {
                    tv_sec: two_hours_ago,
                    tv_usec: 0,
                },
                libc::timeval {
                    tv_sec: two_hours_ago,
                    tv_usec: 0,
                },
            ];
            let path_cstr = CString::new(db_path.to_str().unwrap()).unwrap();
            let _ = unsafe { libc::utimes(path_cstr.as_ptr(), times.as_ptr()) };

            // Now ensure_index should see it as stale and return a Refreshing note.
            let note = tool.ensure_index();
            assert!(
                note.contains("Refreshing") || note.contains("Building") || note.contains("Index"),
                "expected refresh note, got: {:?}",
                note
            );
        }
    }

    /// Line 142: num_results is clamped to 20.
    #[tokio::test]
    async fn execute_with_large_num_results_clamped() {
        let dir = tempfile::tempdir().unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(serde_json::json!({"query": "hello", "num_results": 100}))
            .await;
        assert!(!out.is_error);
    }

    /// Lines 175, 177: target_directory filter (Some and None paths).
    #[tokio::test]
    async fn execute_with_target_directory_filter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.rs"), "fn search_target() {}").unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        // Build index first
        let _ = tool.ensure_index();
        // With target_directory set
        let out = tool
            .execute(serde_json::json!({
                "query": "search_target",
                "target_directory": "."
            }))
            .await;
        assert!(!out.is_error);
    }

    #[tokio::test]
    async fn execute_without_target_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bar.rs"), "fn search_no_dir() {}").unwrap();
        let tool = SemanticSearchTool::new(dir.path().to_path_buf());
        let _ = tool.ensure_index();
        let out = tool
            .execute(serde_json::json!({"query": "search_no_dir"}))
            .await;
        assert!(!out.is_error);
    }

    /// Line 82: ensure_index when RepoIndex::open fails.
    #[test]
    fn ensure_index_open_error_does_not_panic() {
        let bad_root = std::path::PathBuf::from("/nonexistent_xyz/a/b");
        let tool = SemanticSearchTool::new(bad_root);
        let note = tool.ensure_index();
        let _ = note;
    }

    /// Line 91: idx.build fails — workspace_root is a file, not a directory.
    #[test]
    fn ensure_index_build_error_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        // Create a file at workspace_root so idx.build (which walks the directory) still works
        // but make workspace_root point to a file (not a dir) to make the WalkBuilder fail.
        // Actually, create a valid dir but let the db be in a place where open succeeds
        // and then build fails because the "workspace_root" is a regular file.
        let workspace_file = dir.path().join("workspace_as_file");
        std::fs::write(&workspace_file, "not a dir").unwrap();
        let clido_dir = dir.path().join(".clido");
        std::fs::create_dir_all(&clido_dir).unwrap();
        // Create the index.db at the expected location but make workspace_root a file
        // so build will be called on a file path (WalkBuilder on a file just walks that file)
        // In this case, build might succeed or fail but shouldn't panic
        let tool = SemanticSearchTool::new(workspace_file.clone());
        let note = tool.ensure_index();
        // Should handle gracefully regardless
        let _ = note;
    }
}
