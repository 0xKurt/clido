//! `ApplyPatch` tool — apply a unified diff to one or more files.
//!
//! Accepts standard unified diff format (`--- a/file`, `+++ b/file`, `@@ ... @@` hunks).
//! Each hunk is applied independently; if any hunk fails to apply, the entire patch is
//! rejected and the file is left unchanged.

use std::path::{Path, PathBuf};

use crate::{PathGuard, Tool, ToolOutput};

pub struct ApplyPatchTool {
    guard: PathGuard,
}

impl ApplyPatchTool {
    pub fn new(guard: PathGuard) -> Self {
        Self { guard }
    }

    /// Parse and apply a unified diff string.  Returns a list of files modified.
    fn apply(&self, patch: &str) -> Result<Vec<String>, String> {
        let file_patches = parse_unified_diff(patch)?;
        if file_patches.is_empty() {
            return Err("No file patches found in diff".to_string());
        }

        let mut modified = Vec::new();
        for fp in &file_patches {
            let path = if fp.is_new_file {
                self.guard
                    .resolve_for_write(&fp.path)
                    .map_err(|e| format!("{}: {}", fp.path, e))?
            } else {
                self.guard
                    .resolve_and_check(&fp.path)
                    .map_err(|e| format!("{}: {}", fp.path, e))?
            };
            apply_file_patch(&path, &fp.hunks, fp.is_new_file, fp.is_deleted_file)
                .map_err(|e| format!("Failed to patch {}: {}", fp.path, e))?;
            modified.push(fp.path.clone());
        }
        Ok(modified)
    }
}

#[async_trait::async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "ApplyPatch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff (patch) to one or more files. \
         The patch must be in standard unified diff format with --- / +++ headers \
         and @@ hunk markers. Use this to make precise multi-line edits."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "Unified diff to apply. Must include --- / +++ file headers and @@ hunk markers."
                }
            },
            "required": ["patch"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> ToolOutput {
        let patch = match input.get("patch").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return ToolOutput::err("Missing required parameter: patch".to_string()),
        };

        match self.apply(&patch) {
            Ok(files) if files.is_empty() => ToolOutput::ok("No changes applied".to_string()),
            Ok(files) => ToolOutput::ok(format!("Patched: {}", files.join(", "))),
            Err(e) => ToolOutput::err(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Unified diff parser
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct FilePatch {
    path: String,
    is_new_file: bool,
    is_deleted_file: bool,
    hunks: Vec<Hunk>,
}

#[derive(Debug)]
struct Hunk {
    /// Original file start line (1-based).
    orig_start: usize,
    /// Lines in this hunk (context + removes + adds).
    lines: Vec<HunkLine>,
}

#[derive(Debug, Clone)]
enum HunkLine {
    Context(String),
    Remove(String),
    Add(String),
}

fn parse_unified_diff(patch: &str) -> Result<Vec<FilePatch>, String> {
    let mut files: Vec<FilePatch> = Vec::new();
    let mut current: Option<FilePatch> = None;
    let mut current_hunk: Option<Hunk> = None;

    for raw_line in patch.lines() {
        if raw_line.starts_with("--- ") {
            // Flush previous.
            if let Some(mut fp) = current.take() {
                if let Some(h) = current_hunk.take() {
                    fp.hunks.push(h);
                }
                files.push(fp);
            }
            // New file patch starts on +++ line; record "from" path tentatively.
            let from = strip_path_prefix(raw_line.trim_start_matches("--- "));
            current = Some(FilePatch {
                path: from,
                is_new_file: false,
                is_deleted_file: false,
                hunks: Vec::new(),
            });
            current_hunk = None;
        } else if raw_line.starts_with("+++ ") {
            let to = strip_path_prefix(raw_line.trim_start_matches("+++ "));
            if let Some(ref mut fp) = current {
                if to == "/dev/null" {
                    fp.is_deleted_file = true;
                } else if fp.path == "/dev/null" {
                    fp.path = to;
                    fp.is_new_file = true;
                } else {
                    // Use the destination ("+++ ") path as canonical.
                    fp.path = to;
                }
            }
        } else if raw_line.starts_with("@@ ") {
            // Flush current hunk.
            if let Some(ref mut fp) = current {
                if let Some(h) = current_hunk.take() {
                    fp.hunks.push(h);
                }
            }
            let orig_start = parse_hunk_header(raw_line)?;
            current_hunk = Some(Hunk {
                orig_start,
                lines: Vec::new(),
            });
        } else if let Some(ref mut hunk) = current_hunk {
            if raw_line.starts_with(' ') || raw_line.is_empty() {
                hunk.lines.push(HunkLine::Context(
                    raw_line.strip_prefix(' ').unwrap_or("").to_string(),
                ));
            } else if let Some(rest) = raw_line.strip_prefix('-') {
                hunk.lines.push(HunkLine::Remove(rest.to_string()));
            } else if let Some(rest) = raw_line.strip_prefix('+') {
                hunk.lines.push(HunkLine::Add(rest.to_string()));
            }
            // Lines starting with '\' (no newline at end of file) are silently skipped.
        }
    }

    // Flush last.
    if let Some(mut fp) = current {
        if let Some(h) = current_hunk {
            fp.hunks.push(h);
        }
        files.push(fp);
    }

    Ok(files)
}

/// Strip `a/` or `b/` prefix added by `git diff`.
fn strip_path_prefix(s: &str) -> String {
    if let Some(r) = s.strip_prefix("a/").or_else(|| s.strip_prefix("b/")) {
        r.to_string()
    } else {
        s.to_string()
    }
}

/// Parse `@@ -12,7 +12,8 @@` → original start line (1-based).
fn parse_hunk_header(line: &str) -> Result<usize, String> {
    // Format: @@ -<start>[,<count>] +<start>[,<count>] @@
    let inner = line
        .trim_start_matches('@')
        .trim_start()
        .trim_end_matches('@')
        .trim();
    let minus_part = inner
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("Invalid hunk header: {}", line))?;
    let orig = minus_part.trim_start_matches('-');
    let start_str = orig.split(',').next().unwrap_or(orig);
    start_str
        .parse::<usize>()
        .map_err(|_| format!("Cannot parse hunk start from: {}", line))
}

// ---------------------------------------------------------------------------
// File patching
// ---------------------------------------------------------------------------

fn apply_file_patch(
    path: &Path,
    hunks: &[Hunk],
    is_new_file: bool,
    is_deleted_file: bool,
) -> Result<(), String> {
    if is_new_file {
        return apply_new_file(path, hunks);
    }
    if is_deleted_file {
        return apply_delete_file(path);
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    // Apply hunks in reverse order so line numbers don't shift.
    let mut sorted_hunks: Vec<&Hunk> = hunks.iter().collect();
    sorted_hunks.sort_by(|a, b| b.orig_start.cmp(&a.orig_start));

    for hunk in sorted_hunks {
        lines = apply_hunk(lines, hunk)?;
    }

    let new_content = if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    };

    write_atomically(path, &new_content)
}

fn apply_new_file(path: &Path, hunks: &[Hunk]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
    }
    let mut lines: Vec<String> = Vec::new();
    for hunk in hunks {
        for hl in &hunk.lines {
            match hl {
                HunkLine::Add(l) | HunkLine::Context(l) => lines.push(l.clone()),
                HunkLine::Remove(_) => {}
            }
        }
    }
    let content = if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    };
    write_atomically(path, &content)
}

fn apply_delete_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| format!("Cannot delete {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Apply a single hunk to `lines`, returning the updated line vector.
fn apply_hunk(mut lines: Vec<String>, hunk: &Hunk) -> Result<Vec<String>, String> {
    // orig_start is 1-based; convert to 0-based index.
    let start = if hunk.orig_start == 0 {
        0
    } else {
        hunk.orig_start - 1
    };

    // Verify context and remove lines match.
    let mut file_idx = start;
    for hl in &hunk.lines {
        match hl {
            HunkLine::Context(expected) | HunkLine::Remove(expected) => {
                let actual = lines.get(file_idx).ok_or_else(|| {
                    format!(
                        "Hunk at line {} extends past end of file (file has {} lines)",
                        hunk.orig_start,
                        lines.len()
                    )
                })?;
                if actual != expected {
                    return Err(format!(
                        "Hunk context mismatch at line {}: expected {:?}, found {:?}",
                        file_idx + 1,
                        expected,
                        actual
                    ));
                }
                file_idx += 1;
            }
            HunkLine::Add(_) => {}
        }
    }

    // Splice: collect replacement.
    let mut replacement: Vec<String> = Vec::new();
    for hl in &hunk.lines {
        match hl {
            HunkLine::Context(l) | HunkLine::Add(l) => replacement.push(l.clone()),
            HunkLine::Remove(_) => {}
        }
    }

    // Count lines consumed from original.
    let orig_consumed = hunk
        .lines
        .iter()
        .filter(|l| matches!(l, HunkLine::Context(_) | HunkLine::Remove(_)))
        .count();

    lines.splice(start..start + orig_consumed, replacement);
    Ok(lines)
}

/// Write file contents atomically via a temp file rename.
fn write_atomically(path: &Path, content: &str) -> Result<(), String> {
    // Determine temp path in same directory for atomic rename.
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path: PathBuf = dir.join(format!(
        ".clido_patch_tmp_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    ));
    std::fs::write(&tmp_path, content).map_err(|e| format!("Cannot write temp file: {}", e))?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("Cannot rename temp file to {}: {}", path.display(), e)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_guard(dir: &TempDir) -> PathGuard {
        PathGuard::new(dir.path().to_path_buf())
    }

    #[test]
    fn parse_simple_hunk() {
        let patch = "--- a/foo.txt\n+++ b/foo.txt\n@@ -1,3 +1,3 @@\n context\n-old line\n+new line\n context2\n";
        let files = parse_unified_diff(patch).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "foo.txt");
        assert_eq!(files[0].hunks.len(), 1);
        assert_eq!(files[0].hunks[0].orig_start, 1);
    }

    #[test]
    fn apply_replace_line() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line1\nold\nline3\n").unwrap();

        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch = "--- a/test.txt\n+++ b/test.txt\n@@ -1,3 +1,3 @@\n line1\n-old\n+new\n line3\n";
        let result = tool.apply(patch).unwrap();
        assert!(result.contains(&"test.txt".to_string()));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "line1\nnew\nline3\n"
        );
    }

    #[test]
    fn apply_add_lines() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("add.txt");
        std::fs::write(&file, "a\nb\n").unwrap();

        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch = "--- a/add.txt\n+++ b/add.txt\n@@ -1,2 +1,3 @@\n a\n+middle\n b\n";
        tool.apply(patch).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "a\nmiddle\nb\n");
    }

    #[test]
    fn apply_remove_lines() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("rem.txt");
        std::fs::write(&file, "keep\nremove\nkeep2\n").unwrap();

        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch = "--- a/rem.txt\n+++ b/rem.txt\n@@ -1,3 +1,2 @@\n keep\n-remove\n keep2\n";
        tool.apply(patch).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "keep\nkeep2\n");
    }

    #[test]
    fn apply_new_file() {
        let dir = TempDir::new().unwrap();
        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch = "--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1,2 @@\n+hello\n+world\n";
        tool.apply(patch).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "hello\nworld\n"
        );
    }

    #[test]
    fn apply_delete_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("gone.txt");
        std::fs::write(&file, "bye\n").unwrap();

        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch = "--- a/gone.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-bye\n";
        tool.apply(patch).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn context_mismatch_returns_error() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("mismatch.txt");
        std::fs::write(&file, "actual content\n").unwrap();

        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch =
            "--- a/mismatch.txt\n+++ b/mismatch.txt\n@@ -1 +1 @@\n-wrong context\n+replacement\n";
        let err = tool.apply(patch).unwrap_err();
        assert!(
            err.contains("mismatch"),
            "Expected mismatch error, got: {}",
            err
        );
        // File must be unchanged.
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "actual content\n");
    }

    #[test]
    fn multi_file_patch() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "world\n").unwrap();

        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let patch = concat!(
            "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-hello\n+hi\n",
            "--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-world\n+earth\n"
        );
        let modified = tool.apply(patch).unwrap();
        assert_eq!(modified.len(), 2);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "hi\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.txt")).unwrap(),
            "earth\n"
        );
    }

    #[test]
    fn empty_patch_returns_error() {
        let dir = TempDir::new().unwrap();
        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        assert!(tool.apply("").unwrap_err().contains("No file patches"));
    }

    #[tokio::test]
    async fn execute_missing_patch_param() {
        let dir = TempDir::new().unwrap();
        let guard = make_guard(&dir);
        let tool = ApplyPatchTool::new(guard);
        let out = tool.execute(serde_json::json!({})).await;
        assert!(out.is_error);
    }
}
