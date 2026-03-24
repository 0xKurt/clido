//! Append-only audit log for tool calls.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub session_id: String,
    pub tool_name: String,
    pub input_summary: String,
    pub is_error: bool,
    pub duration_ms: u64,
}

pub struct AuditLog {
    file: std::fs::File,
}

impl AuditLog {
    /// Open (or create) the audit log for a project.
    pub fn open(project_path: &Path) -> anyhow::Result<Self> {
        let dir = crate::paths::data_dir()?
            .join("audit")
            .join(crate::paths::sanitize_for_audit(project_path));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("audit.jsonl");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self { file })
    }

    pub fn append(&mut self, entry: &AuditEntry) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.file, entry)?;
        self.file.write_all(b"\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_roundtrip() {
        let entry = AuditEntry {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            session_id: "sess-abc".to_string(),
            tool_name: "Read".to_string(),
            input_summary: "path=/tmp/foo.rs".to_string(),
            is_error: false,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, entry.session_id);
        assert_eq!(back.tool_name, entry.tool_name);
        assert_eq!(back.duration_ms, 42);
        assert!(!back.is_error);
    }

    #[test]
    fn audit_log_open_and_append() {
        let dir = tempfile::tempdir().unwrap();
        let project_path = dir.path();
        let mut log = AuditLog::open(project_path).unwrap();
        let entry = AuditEntry {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            session_id: "s1".to_string(),
            tool_name: "Write".to_string(),
            input_summary: "path=/tmp/out.rs".to_string(),
            is_error: false,
            duration_ms: 100,
        };
        log.append(&entry).unwrap();
        // AuditLog::open puts the file in the data_dir/audit/<sanitized_path>/audit.jsonl.
        // For tests we just verify append doesn't panic and returns Ok.
        // The actual location depends on the platform data dir, so just confirm no error.
    }

    #[test]
    fn audit_log_appends_multiple_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = AuditLog::open(dir.path()).unwrap();
        for i in 0..3 {
            let entry = AuditEntry {
                timestamp: format!("2026-01-01T00:00:0{}Z", i),
                session_id: "s2".to_string(),
                tool_name: format!("Tool{}", i),
                input_summary: format!("input{}", i),
                is_error: i == 2,
                duration_ms: i as u64 * 10,
            };
            log.append(&entry).unwrap();
        }
        // If we get here without panic, all 3 appends succeeded.
    }
}
