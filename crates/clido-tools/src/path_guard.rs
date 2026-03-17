//! Path canonicalization and workspace root check (security-model §1–3).

use std::path::{Path, PathBuf};

/// Canonicalize path and ensure it is under `workspace_root`. Returns error content for tool output.
pub fn resolve_and_check(path: &str, workspace_root: &Path) -> Result<PathBuf, String> {
    let root_canon = std::fs::canonicalize(workspace_root).map_err(|e| e.to_string())?;
    let joined = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        root_canon.join(path)
    };
    let normalized = normalize_path(&joined);
    if !normalized.starts_with(&root_canon) {
        return Err("Access denied: path outside working directory.".to_string());
    }
    let canonical = match std::fs::canonicalize(&normalized) {
        Ok(p) => p,
        Err(e) => return Err(e.to_string()),
    };
    if !canonical.starts_with(&root_canon) {
        return Err("Access denied: path outside working directory.".to_string());
    }
    Ok(canonical)
}

fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => out.push(c),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// Resolve path for write: file may not exist yet. Check under workspace_root.
pub fn resolve_for_write(path: &str, workspace_root: &Path) -> Result<PathBuf, String> {
    let root_canon = std::fs::canonicalize(workspace_root).map_err(|e| e.to_string())?;
    let joined = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        root_canon.join(path)
    };
    if joined.exists() {
        let canonical = std::fs::canonicalize(&joined).map_err(|e| e.to_string())?;
        if !canonical.starts_with(&root_canon) {
            return Err("Access denied: path outside working directory.".to_string());
        }
        return Ok(canonical);
    }
    if let Some(parent) = joined.parent() {
        let canon_parent = match std::fs::canonicalize(parent) {
            Ok(p) => p,
            Err(_) => {
                if parent == root_canon || parent.starts_with(&root_canon) {
                    return Ok(joined);
                }
                return Err("Access denied: path outside working directory.".to_string());
            }
        };
        if !canon_parent.starts_with(&root_canon) {
            return Err("Access denied: path outside working directory.".to_string());
        }
        if let Some(name) = joined.file_name() {
            return Ok(canon_parent.join(name));
        }
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_under_root() {
        let tmp = std::env::temp_dir().join("clido_path_guard_test_1");
        let _ = std::fs::create_dir_all(&tmp);
        let f = tmp.join("a").join("b.txt");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, "x").unwrap();
        let res = resolve_and_check("a/b.txt", &tmp);
        assert!(res.is_ok());
        assert!(res.unwrap().ends_with("b.txt"));
    }

    #[test]
    fn path_outside_root_denied() {
        let tmp = std::env::temp_dir().join("clido_path_guard_test_2");
        std::fs::create_dir_all(&tmp).unwrap();
        let res = resolve_and_check("../../../etc/passwd", &tmp);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Access denied"));
    }
}
