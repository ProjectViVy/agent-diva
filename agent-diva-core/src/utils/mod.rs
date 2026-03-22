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
const DEFAULT_SOUL_MD: &str = r#"# Soul

## Core Traits
- Keep responses helpful, direct, and reliable.
- Prioritize user intent and long-term consistency.

## Boundaries
- Do not fabricate facts.
- Be explicit when uncertain.

## Evolution Notes
- Record stable behavioral refinements here.
"#;
const DEFAULT_IDENTITY_MD: &str = r#"# Identity

- Name: Agent Diva
- Role: Modular AI assistant
- Voice: Concise, practical, collaborative
"#;
const DEFAULT_USER_MD: &str = r#"# User Profile

## Preferences
- Keep this file for durable user communication preferences.

## Collaboration Norms
- Prefer transparent reasoning and concise action summaries.
"#;
const DEFAULT_BOOTSTRAP_MD: &str = r#"# Bootstrap

You just came online. Use this first conversation to shape your identity.

## Conversation goals
1. Learn what the user wants to call you (name and optional emoji).
2. Clarify preferred collaboration style (concise vs detailed, directness, language).
3. Clarify boundaries: what must be asked first, what should never be done.
4. Capture user profile details that improve future collaboration.

## Required updates
- Update `IDENTITY.md` with name, role, voice, and emoji.
- Update `USER.md` with durable user preferences.
- Update `SOUL.md` with refined boundaries and behavior principles.

## Completion
- Tell the user onboarding is complete.
- Mark bootstrap as completed in soul state or remove this file.
- If this workspace has `docs/dev/archive/architecture-reports/soul-mechanism-analysis.md`, treat it as the primary soul-architecture reference when implementing related development tasks.
"#;

/// Sync workspace templates. Missing files are created; existing files are never overwritten.
pub fn sync_workspace_templates<P: AsRef<Path>>(workspace: P) -> std::io::Result<Vec<String>> {
    let workspace = workspace.as_ref();
    std::fs::create_dir_all(workspace)?;
    std::fs::create_dir_all(workspace.join("memory"))?;
    std::fs::create_dir_all(workspace.join("skills"))?;

    let mut added = Vec::new();
    let templates: [(&str, Option<&str>); 8] = [
        ("memory/MEMORY.md", Some(DEFAULT_MEMORY_MD)),
        ("memory/HISTORY.md", None),
        ("PROFILE.md", Some(DEFAULT_PROFILE_MD)),
        ("SOUL.md", Some(DEFAULT_SOUL_MD)),
        ("IDENTITY.md", Some(DEFAULT_IDENTITY_MD)),
        ("USER.md", Some(DEFAULT_USER_MD)),
        ("BOOTSTRAP.md", Some(DEFAULT_BOOTSTRAP_MD)),
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
        assert!(temp.path().join("SOUL.md").exists());
        assert!(temp.path().join("IDENTITY.md").exists());
        assert!(temp.path().join("USER.md").exists());
        assert!(temp.path().join("BOOTSTRAP.md").exists());
        assert!(temp.path().join("skills").exists());
    }

    #[test]
    fn test_sync_workspace_templates_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let first = sync_workspace_templates(temp.path()).unwrap();
        assert!(!first.is_empty());

        let second = sync_workspace_templates(temp.path()).unwrap();
        assert!(second.is_empty());
    }

    #[test]
    fn test_sync_workspace_templates_does_not_overwrite_existing_file() {
        let temp = tempfile::tempdir().unwrap();
        let soul_path = temp.path().join("SOUL.md");
        std::fs::write(&soul_path, "# Soul\n\ncustom content\n").unwrap();

        let _ = sync_workspace_templates(temp.path()).unwrap();
        let current = std::fs::read_to_string(&soul_path).unwrap();
        assert_eq!(current, "# Soul\n\ncustom content\n");
    }
}
