//! Session migration from Python to Rust format
//!
//! Both versions use JSONL format, so this is mainly a copy operation
//! with validation and progress reporting.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Result of session migration
#[derive(Debug, Default)]
pub struct MigrationResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub sessions: Vec<SessionInfo>,
}

/// Information about a migrated session
#[derive(Debug)]
#[allow(dead_code)]
pub struct SessionInfo {
    pub key: String,
    pub message_count: usize,
    pub path: PathBuf,
}

/// Session migrator
pub struct SessionMigrator {
    source_dir: PathBuf,
    target_dir: PathBuf,
}

impl SessionMigrator {
    /// Create a new session migrator
    pub fn new(source_dir: impl AsRef<Path>, target_dir: impl AsRef<Path>) -> Self {
        Self {
            source_dir: source_dir.as_ref().join("sessions"),
            target_dir: target_dir.as_ref().join("sessions"),
        }
    }

    /// Migrate sessions from Python to Rust format
    pub async fn migrate(&self, dry_run: bool) -> Result<MigrationResult> {
        let mut result = MigrationResult::default();

        // Check if source sessions directory exists
        if !self.source_dir.exists() {
            info!(
                "No source sessions directory found at {:?}",
                self.source_dir
            );
            return Ok(result);
        }

        // Create target directory if needed
        if !dry_run {
            tokio::fs::create_dir_all(&self.target_dir)
                .await
                .with_context(|| {
                    format!(
                        "Failed to create target sessions directory: {:?}",
                        self.target_dir
                    )
                })?;
        }

        // Read all session files
        let mut entries = tokio::fs::read_dir(&self.source_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to read source sessions directory: {:?}",
                    self.source_dir
                )
            })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .jsonl files
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            result.total += 1;

            match self.migrate_session(&path, dry_run).await {
                Ok(info) => {
                    result.successful += 1;
                    result.sessions.push(info);
                }
                Err(e) => {
                    result.failed += 1;
                    error!("Failed to migrate session {:?}: {}", path, e);
                }
            }
        }

        info!(
            "Session migration complete: {}/{} successful",
            result.successful, result.total
        );

        Ok(result)
    }

    /// Migrate a single session file
    async fn migrate_session(&self, source_path: &Path, dry_run: bool) -> Result<SessionInfo> {
        // Get the filename
        let filename = source_path
            .file_name()
            .context("Invalid session file path")?;

        let target_path = self.target_dir.join(filename);

        // Read and validate the session file
        let content = tokio::fs::read_to_string(source_path)
            .await
            .with_context(|| format!("Failed to read session file: {:?}", source_path))?;

        // Validate JSONL format and count messages
        let mut message_count = 0;

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Try to parse as JSON
            let json: serde_json::Value = serde_json::from_str(line).with_context(|| {
                format!("Invalid JSON on line {} in {:?}", line_num + 1, source_path)
            })?;

            // Check if it's metadata
            if json.get("_type").and_then(|t| t.as_str()) == Some("metadata") {
                // Metadata line, skip counting
            } else {
                // Validate message structure
                if json.get("role").is_some() && json.get("content").is_some() {
                    message_count += 1;
                }
            }
        }

        // Extract session key from filename (convert _ back to :)
        let key = filename
            .to_string_lossy()
            .trim_end_matches(".jsonl")
            .replace('_', ":");

        if dry_run {
            info!(
                "Dry run: would migrate session {} with {} messages",
                key, message_count
            );
            return Ok(SessionInfo {
                key,
                message_count,
                path: target_path,
            });
        }

        // Copy the file
        tokio::fs::copy(source_path, &target_path)
            .await
            .with_context(|| format!("Failed to copy session file to {:?}", target_path))?;

        info!("Migrated session {} with {} messages", key, message_count);

        Ok(SessionInfo {
            key,
            message_count,
            path: target_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_migration() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let source_sessions = source_dir.join("sessions");

        tokio::fs::create_dir_all(&source_sessions).await.unwrap();

        // Create a sample session file
        let session_content = r#"{"_type":"metadata","created_at":"2024-01-01T00:00:00","updated_at":"2024-01-01T01:00:00","metadata":{}}
{"role":"user","content":"Hello","timestamp":"2024-01-01T00:00:00"}
{"role":"assistant","content":"Hi there!","timestamp":"2024-01-01T00:01:00"}
"#;

        tokio::fs::write(
            source_sessions.join("telegram_12345.jsonl"),
            session_content,
        )
        .await
        .unwrap();

        let migrator = SessionMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.sessions[0].key, "telegram:12345");
        assert_eq!(result.sessions[0].message_count, 2);

        // Verify the file was copied
        let target_path = target_dir.join("sessions").join("telegram_12345.jsonl");
        assert!(target_path.exists());
    }

    #[tokio::test]
    async fn test_session_migration_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let source_sessions = source_dir.join("sessions");

        tokio::fs::create_dir_all(&source_sessions).await.unwrap();

        tokio::fs::write(source_sessions.join("test.jsonl"), "{}")
            .await
            .unwrap();

        let migrator = SessionMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(true).await.unwrap();

        assert_eq!(result.total, 1);

        // Verify the file was NOT copied
        let target_path = target_dir.join("sessions").join("test.jsonl");
        assert!(!target_path.exists());
    }

    #[tokio::test]
    async fn test_no_source_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        tokio::fs::create_dir_all(&source_dir).await.unwrap();

        let migrator = SessionMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.successful, 0);
    }

    #[tokio::test]
    async fn test_invalid_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let source_sessions = source_dir.join("sessions");

        tokio::fs::create_dir_all(&source_sessions).await.unwrap();

        // Create an invalid session file
        tokio::fs::write(source_sessions.join("invalid.jsonl"), "not valid json")
            .await
            .unwrap();

        let migrator = SessionMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 1);
    }
}
