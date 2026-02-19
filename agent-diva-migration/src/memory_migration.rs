//! Memory migration from Python to Rust format
//!
//! Memory files are Markdown format, so this is mainly a copy operation
//! for daily notes and MEMORY.md files.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Result of memory migration
#[derive(Debug, Default)]
pub struct MigrationResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub files: Vec<MemoryFileInfo>,
}

/// Information about a migrated memory file
#[derive(Debug)]
#[allow(dead_code)]
pub struct MemoryFileInfo {
    pub filename: String,
    pub size_bytes: u64,
    pub is_daily_note: bool,
    pub path: PathBuf,
}

/// Memory migrator
pub struct MemoryMigrator {
    source_dir: PathBuf,
    target_dir: PathBuf,
}

impl MemoryMigrator {
    /// Create a new memory migrator
    pub fn new(source_dir: impl AsRef<Path>, target_dir: impl AsRef<Path>) -> Self {
        // Memory is stored in workspace/memory directory
        Self {
            source_dir: source_dir.as_ref().to_path_buf(),
            target_dir: target_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the source memory directory path
    fn source_memory_dir(&self) -> PathBuf {
        self.source_dir.join("workspace").join("memory")
    }

    /// Get the target memory directory path
    fn target_memory_dir(&self) -> PathBuf {
        self.target_dir.join("workspace").join("memory")
    }

    /// Migrate memory files from Python to Rust format
    pub async fn migrate(&self, dry_run: bool) -> Result<MigrationResult> {
        let mut result = MigrationResult::default();
        let source_memory = self.source_memory_dir();

        // Check if source memory directory exists
        if !source_memory.exists() {
            info!("No source memory directory found at {:?}", source_memory);
            return Ok(result);
        }

        // Create target directory if needed
        let target_memory = self.target_memory_dir();
        if !dry_run {
            tokio::fs::create_dir_all(&target_memory)
                .await
                .with_context(|| {
                    format!(
                        "Failed to create target memory directory: {:?}",
                        target_memory
                    )
                })?;
        }

        // Read all memory files
        let mut entries = tokio::fs::read_dir(&source_memory).await.with_context(|| {
            format!(
                "Failed to read source memory directory: {:?}",
                source_memory
            )
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .md files
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            result.total += 1;

            match self.migrate_memory_file(&path, dry_run).await {
                Ok(info) => {
                    result.successful += 1;
                    result.files.push(info);
                }
                Err(e) => {
                    result.failed += 1;
                    error!("Failed to migrate memory file {:?}: {}", path, e);
                }
            }
        }

        info!(
            "Memory migration complete: {}/{} successful",
            result.successful, result.total
        );

        Ok(result)
    }

    /// Migrate a single memory file
    async fn migrate_memory_file(
        &self,
        source_path: &Path,
        dry_run: bool,
    ) -> Result<MemoryFileInfo> {
        // Get the filename
        let filename = source_path
            .file_name()
            .context("Invalid memory file path")?
            .to_string_lossy()
            .to_string();

        let target_path = self.target_memory_dir().join(&filename);

        // Get file metadata
        let metadata = tokio::fs::metadata(source_path)
            .await
            .with_context(|| format!("Failed to read metadata for: {:?}", source_path))?;

        let size_bytes = metadata.len();

        // Determine if it's a daily note (YYYY-MM-DD.md format) or MEMORY.md
        let is_daily_note = {
            let stem = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            // Check if it matches YYYY-MM-DD pattern
            stem.len() == 10
                && stem.chars().nth(4) == Some('-')
                && stem.chars().nth(7) == Some('-')
                && stem.chars().all(|c| c.is_ascii_digit() || c == '-')
        };

        if dry_run {
            info!(
                "Dry run: would migrate memory file {} ({} bytes)",
                filename, size_bytes
            );
            return Ok(MemoryFileInfo {
                filename,
                size_bytes,
                is_daily_note,
                path: target_path,
            });
        }

        // Copy the file
        tokio::fs::copy(source_path, &target_path)
            .await
            .with_context(|| format!("Failed to copy memory file to {:?}", target_path))?;

        info!("Migrated memory file {} ({} bytes)", filename, size_bytes);

        Ok(MemoryFileInfo {
            filename,
            size_bytes,
            is_daily_note,
            path: target_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_migration() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let source_memory = source_dir.join("workspace").join("memory");

        tokio::fs::create_dir_all(&source_memory).await.unwrap();

        // Create MEMORY.md
        tokio::fs::write(
            source_memory.join("MEMORY.md"),
            "# Long-term Memory\n\nTest content",
        )
        .await
        .unwrap();

        // Create a daily note
        tokio::fs::write(
            source_memory.join("2024-01-15.md"),
            "# 2024-01-15\n\nDaily notes",
        )
        .await
        .unwrap();

        let migrator = MemoryMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);

        // Check file classifications
        let memory_md = result
            .files
            .iter()
            .find(|f| f.filename == "MEMORY.md")
            .unwrap();
        assert!(!memory_md.is_daily_note);

        let daily_note = result
            .files
            .iter()
            .find(|f| f.filename == "2024-01-15.md")
            .unwrap();
        assert!(daily_note.is_daily_note);

        // Verify files were copied
        let target_memory = target_dir.join("workspace").join("memory");
        assert!(target_memory.join("MEMORY.md").exists());
        assert!(target_memory.join("2024-01-15.md").exists());
    }

    #[tokio::test]
    async fn test_memory_migration_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let source_memory = source_dir.join("workspace").join("memory");

        tokio::fs::create_dir_all(&source_memory).await.unwrap();

        tokio::fs::write(source_memory.join("MEMORY.md"), "test")
            .await
            .unwrap();

        let migrator = MemoryMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(true).await.unwrap();

        assert_eq!(result.total, 1);

        // Verify the file was NOT copied
        let target_memory = target_dir.join("workspace").join("memory");
        assert!(!target_memory.join("MEMORY.md").exists());
    }

    #[tokio::test]
    async fn test_no_source_memory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        tokio::fs::create_dir_all(&source_dir).await.unwrap();

        let migrator = MemoryMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.successful, 0);
    }

    #[tokio::test]
    async fn test_non_md_files_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let source_memory = source_dir.join("workspace").join("memory");

        tokio::fs::create_dir_all(&source_memory).await.unwrap();

        // Create a non-markdown file
        tokio::fs::write(source_memory.join("notes.txt"), "test")
            .await
            .unwrap();

        tokio::fs::write(source_memory.join("MEMORY.md"), "test")
            .await
            .unwrap();

        let migrator = MemoryMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        // Only .md files should be migrated
        assert_eq!(result.total, 1);
        assert_eq!(result.files[0].filename, "MEMORY.md");
    }
}
