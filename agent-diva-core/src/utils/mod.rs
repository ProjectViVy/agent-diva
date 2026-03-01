//! Utility functions and helpers

use std::path::Path;

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> std::path::PathBuf {
    let path = path.as_ref();
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
    path.to_path_buf()
}

/// Create a safe filename from a string
pub fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

/// Truncate a string to a maximum byte length, ensuring valid UTF-8 boundaries
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(3);
        while !s.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        format!("{}...", &s[..end])
    }
}

const DEFAULT_MEMORY_MD: &str = "# Long-term Memory\n\nRecord durable facts here.\n";
const DEFAULT_PROFILE_MD: &str = "# Profile\n\n- Name:\n- Preferences:\n";

/// Sync workspace templates. Missing files are created; existing files are never overwritten.
pub fn sync_workspace_templates<P: AsRef<Path>>(workspace: P) -> std::io::Result<Vec<String>> {
    let workspace = workspace.as_ref();
    std::fs::create_dir_all(workspace)?;
    std::fs::create_dir_all(workspace.join("memory"))?;
    std::fs::create_dir_all(workspace.join("skills"))?;

    let mut added = Vec::new();
    let templates: [(&str, Option<&str>); 4] = [
        ("memory/MEMORY.md", Some(DEFAULT_MEMORY_MD)),
        ("memory/HISTORY.md", None),
        ("PROFILE.md", Some(DEFAULT_PROFILE_MD)),
        ("TASK.md", Some("# Tasks\n\n")),
    ];

    for (rel, content) in templates {
        let path = workspace.join(rel);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = content.unwrap_or("");
        std::fs::write(&path, body)?;
        added.push(rel.to_string());
    }

    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_filename() {
        assert_eq!(safe_filename("hello world"), "hello_world");
        assert_eq!(safe_filename("test/file:name"), "test_file_name");
        assert_eq!(safe_filename("normal-name.txt"), "normal-name.txt");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("test", 3), "...");
    }

    #[test]
    fn test_sync_workspace_templates_creates_missing_files() {
        let temp = tempfile::tempdir().unwrap();
        let added = sync_workspace_templates(temp.path()).unwrap();
        assert!(added.contains(&"memory/MEMORY.md".to_string()));
        assert!(temp.path().join("memory").join("HISTORY.md").exists());
        assert!(temp.path().join("skills").exists());
    }
}
