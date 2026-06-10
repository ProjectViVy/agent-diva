//! Session manager for handling multiple sessions

use super::store::Session;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionLoadError {
    #[error("Failed to read session file '{path}': {error}")]
    Unreadable { path: PathBuf, error: String },
    #[error("Failed to parse session file '{path}' at line {line}: {error}")]
    Parse {
        path: PathBuf,
        line: usize,
        error: String,
    },
}

/// Manages conversation sessions
#[derive(Debug)]
pub struct SessionManager {
    /// Sessions directory
    sessions_dir: PathBuf,
    /// In-memory cache of sessions
    cache: HashMap<String, Session>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new<P: AsRef<Path>>(workspace: P) -> Self {
        let sessions_dir = workspace.as_ref().join("sessions");
        Self {
            sessions_dir,
            cache: HashMap::new(),
        }
    }

    /// Get or create a session
    pub fn get_or_create(&mut self, key: impl Into<String>) -> crate::Result<&mut Session> {
        let key = key.into();

        if !self.cache.contains_key(&key) {
            let session = self.load(&key)?.unwrap_or_else(|| Session::new(&key));
            self.cache.insert(key.clone(), session);
        }

        Ok(self.cache.get_mut(&key).unwrap())
    }

    /// Get a session if it exists
    pub fn get(&self, key: &str) -> Option<&Session> {
        self.cache.get(key)
    }

    /// Get a session if it exists (cache or disk). Does not create.
    pub fn get_or_load(&mut self, key: &str) -> crate::Result<Option<&Session>> {
        if !self.cache.contains_key(key) {
            if let Some(session) = self.load(key)? {
                self.cache.insert(key.to_string(), session);
            } else {
                return Ok(None);
            }
        }
        Ok(self.cache.get(key))
    }

    /// Load a session from disk
    fn load(&self, key: &str) -> Result<Option<Session>, SessionLoadError> {
        let path = self.session_path(key);
        let backup_path = self.backup_path(key);
        let path_to_read = if path.exists() {
            path
        } else if backup_path.exists() {
            backup_path
        } else {
            return Ok(None);
        };

        let content = std::fs::read_to_string(&path_to_read).map_err(|error| {
            SessionLoadError::Unreadable {
                path: path_to_read.clone(),
                error: error.to_string(),
            }
        })?;
        let mut messages = Vec::new();
        let mut metadata = serde_json::Value::Object(serde_json::Map::new());
        let mut created_at = None;
        let mut updated_at = None;
        let mut last_consolidated: usize = 0;

        for (line_index, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let value = serde_json::from_str::<serde_json::Value>(line).map_err(|error| {
                SessionLoadError::Parse {
                    path: path_to_read.clone(),
                    line: line_index + 1,
                    error: error.to_string(),
                }
            })?;

            if value.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                metadata = value.get("metadata").cloned().unwrap_or(metadata);
                created_at = value
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok());
                updated_at = value
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok());
                last_consolidated = value
                    .get("last_consolidated")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
            } else {
                let msg = serde_json::from_value::<super::store::ChatMessage>(value).map_err(
                    |error| SessionLoadError::Parse {
                        path: path_to_read.clone(),
                        line: line_index + 1,
                        error: error.to_string(),
                    },
                )?;
                messages.push(msg);
            }
        }

        Ok(Some(Session {
            key: key.to_string(),
            messages,
            created_at: created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: updated_at.unwrap_or_else(chrono::Utc::now),
            metadata,
            last_consolidated,
        }))
    }

    /// Save a session to disk
    pub fn save(&self, session: &Session) -> crate::Result<()> {
        std::fs::create_dir_all(&self.sessions_dir)?;
        let path = self.session_path(&session.key);

        let mut lines = Vec::new();

        // Write metadata
        let metadata = serde_json::json!({
            "_type": "metadata",
            "created_at": session.created_at.to_rfc3339(),
            "updated_at": session.updated_at.to_rfc3339(),
            "metadata": session.metadata,
            "last_consolidated": session.last_consolidated,
        });
        lines.push(serde_json::to_string(&metadata)?);

        // Write messages
        for msg in &session.messages {
            lines.push(serde_json::to_string(msg)?);
        }

        self.write_session_atomically(&path, lines.join("\n").as_bytes())?;
        Ok(())
    }

    /// Delete a session
    pub fn delete(&mut self, key: &str) -> crate::Result<bool> {
        self.cache.remove(key);

        let path = self.session_path(key);
        if path.exists() {
            std::fs::remove_file(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Archive an existing session and clear it from memory, forcing a fresh start
    pub fn archive_and_reset(&mut self, key: &str) -> crate::Result<bool> {
        self.cache.remove(key);

        let path = self.session_path(key);
        if path.exists() {
            let safe_key = key.replace([':', '/', '\\'], "_");
            let timestamp = chrono::Utc::now().timestamp_millis();
            let archive_filename = format!("{}.reset.{}.jsonl", safe_key, timestamp);
            let archive_path = self.sessions_dir.join(archive_filename);

            std::fs::rename(&path, &archive_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".jsonl") {
                        let key = name.trim_end_matches(".jsonl").replace('_', ":");
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Some(first_line) = content.lines().next() {
                                if let Ok(value) =
                                    serde_json::from_str::<serde_json::Value>(first_line)
                                {
                                    if value.get("_type").and_then(|v| v.as_str())
                                        == Some("metadata")
                                    {
                                        sessions.push(SessionInfo {
                                            key,
                                            created_at: value
                                                .get("created_at")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            updated_at: value
                                                .get("updated_at")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            path: entry.path().to_string_lossy().to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    /// Get the file path for a session
    fn session_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace([':', '/', '\\'], "_");
        self.sessions_dir.join(format!("{}.jsonl", safe_key))
    }

    fn backup_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace([':', '/', '\\'], "_");
        self.sessions_dir.join(format!("{}.jsonl.bak", safe_key))
    }

    fn write_session_atomically(&self, path: &Path, content: &[u8]) -> crate::Result<()> {
        crate::utils::atomic_write(path, content)
    }
}

/// Information about a session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    /// Session key
    pub key: String,
    /// Creation time
    pub created_at: Option<String>,
    /// Last update time
    pub updated_at: Option<String>,
    /// File path
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::FileAttachmentRef;
    use crate::session::ChatMessage;
    use tempfile::TempDir;

    #[test]
    fn test_session_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());
        assert!(manager.list_sessions().is_empty());
    }

    #[test]
    fn test_get_or_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("telegram:123").unwrap();
        session.add_message("user", "Hello");

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.key, "telegram:123");
    }

    #[test]
    fn test_save_and_load_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Create and modify session
        let session = manager.get_or_create("test:456").unwrap();
        session.add_message("user", "Test message");
        let key = session.key.clone();

        // Save the session
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Clear cache and reload
        manager.cache.clear();
        let session = manager.get_or_create("test:456").unwrap();

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Test message");
    }

    #[test]
    fn test_archive_and_reset_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Create and modify session
        let session = manager.get_or_create("archive:789").unwrap();
        session.add_message("user", "Message to be archived");
        let key = session.key.clone();

        // Save it so it exists on disk
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Archive it
        let archived = manager.archive_and_reset(&key).unwrap();
        assert!(archived);

        // Check it's removed from cache
        assert!(manager.cache.get(&key).is_none());

        // Get or create should now be empty
        let new_session = manager.get_or_create("archive:789").unwrap();
        assert_eq!(new_session.messages.len(), 0);

        // Check if the original file is gone but there's a file with .reset. in it
        let mut reset_files_count = 0;
        for entry in std::fs::read_dir(temp_dir.path().join("sessions")).unwrap() {
            let entry = entry.unwrap();
            let file_name = entry.file_name().into_string().unwrap();
            if file_name.contains(".reset.") {
                reset_files_count += 1;
            } else if file_name == "archive_789.jsonl" {
                // Should not find the active file since it wasn't saved yet
                panic!("Original file still exists!");
            }
        }
        assert_eq!(
            reset_files_count, 1,
            "Should have exactly one archived file"
        );
    }

    #[test]
    fn test_get_or_load_cache_hit() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("gui:chat-1").unwrap();
        session.add_message("user", "Hello");
        let key = session.key.clone();
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Session is in cache; get_or_load should return it
        let loaded = manager.get_or_load("gui:chat-1").unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().key, "gui:chat-1");
        assert_eq!(loaded.unwrap().messages.len(), 1);
    }

    #[test]
    fn test_get_or_load_disk_exists_cache_miss() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Create and save session
        let session = manager.get_or_create("gui:chat-2").unwrap();
        session.add_message("user", "From disk");
        let key = session.key.clone();
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Clear cache to simulate "not loaded this run"
        manager.cache.clear();

        // get_or_load should load from disk
        let loaded = manager.get_or_load("gui:chat-2").unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().key, "gui:chat-2");
        assert_eq!(loaded.unwrap().messages[0].content, "From disk");
    }

    #[test]
    fn test_get_or_load_disk_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Session never created; no file on disk
        let loaded = manager.get_or_load("gui:nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_and_load_session_with_attachment_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("gui:attachments").unwrap();
        session.add_full_message(ChatMessage::with_attachments(
            "user",
            "please inspect this",
            vec![FileAttachmentRef {
                file_id: "sha256:image123".to_string(),
                filename: "image.png".to_string(),
                mime_type: Some("image/png".to_string()),
                size: 4096,
            }],
        ));
        let key = session.key.clone();

        manager.save(&manager.cache.get(&key).unwrap()).unwrap();
        let content = std::fs::read_to_string(manager.session_path(&key)).unwrap();
        assert!(content.contains("\"attachments\""));
        assert!(content.contains("\"file_id\":\"sha256:image123\""));
        assert!(content.contains("\"filename\":\"image.png\""));
        assert!(content.contains("\"mime_type\":\"image/png\""));
        assert!(content.contains("\"size\":4096"));
        assert!(!content.contains("base64"));
        assert!(!content.contains("bytes"));
        assert!(!content.contains("preview"));

        manager.cache.clear();
        let loaded = manager.get_or_create(&key).unwrap();
        assert_eq!(loaded.messages.len(), 1);
        let attachment = &loaded.messages[0].attachments.as_ref().unwrap()[0];
        assert_eq!(attachment.file_id, "sha256:image123");
        assert_eq!(attachment.filename, "image.png");
        assert_eq!(attachment.mime_type, Some("image/png".to_string()));
        assert_eq!(attachment.size, 4096);
    }

    #[test]
    fn test_load_uses_backup_when_primary_missing() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("gui:backup").unwrap();
        session.add_message("user", "from backup");
        let key = session.key.clone();
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        let primary_path = manager.session_path(&key);
        let backup_path = manager.backup_path(&key);
        std::fs::rename(&primary_path, &backup_path).unwrap();
        manager.cache.clear();

        let loaded = manager.get_or_load(&key).unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "from backup");
    }

    #[test]
    fn test_get_or_load_reports_parse_errors() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path());
        let path = manager.session_path("gui:broken");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{not json").unwrap();

        let error = manager.load("gui:broken").unwrap_err();
        assert!(matches!(error, SessionLoadError::Parse { .. }));
    }
}
