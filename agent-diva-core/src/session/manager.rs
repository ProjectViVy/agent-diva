//! Session manager for handling multiple sessions

use super::store::Session;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    pub fn get_or_create(&mut self, key: impl Into<String>) -> &mut Session {
        let key = key.into();

        if !self.cache.contains_key(&key) {
            let session = self.load(&key).unwrap_or_else(|| Session::new(&key));
            self.cache.insert(key.clone(), session);
        }

        self.cache.get_mut(&key).unwrap()
    }

    /// Get a session if it exists
    pub fn get(&self, key: &str) -> Option<&Session> {
        self.cache.get(key)
    }

    /// Get a session if it exists (cache or disk). Does not create.
    pub fn get_or_load(&mut self, key: &str) -> Option<&Session> {
        if !self.cache.contains_key(key) {
            if let Some(session) = self.load(key) {
                self.cache.insert(key.to_string(), session);
            } else {
                return None;
            }
        }
        self.cache.get(key)
    }

    /// Load a session from disk
    fn load(&self, key: &str) -> Option<Session> {
        let path = self.session_path(key);

        if !path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        let mut messages = Vec::new();
        let mut metadata = serde_json::Value::Object(serde_json::Map::new());
        let mut created_at = None;
        let mut last_consolidated: usize = 0;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if value.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                    metadata = value.get("metadata").cloned().unwrap_or(metadata);
                    created_at = value
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok());
                    last_consolidated = value
                        .get("last_consolidated")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                } else if let Ok(msg) = serde_json::from_value::<super::store::ChatMessage>(value) {
                    messages.push(msg);
                }
            }
        }

        Some(Session {
            key: key.to_string(),
            messages,
            created_at: created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: chrono::Utc::now(),
            metadata,
            last_consolidated,
        })
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

        std::fs::write(&path, lines.join("\n"))?;
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

        let session = manager.get_or_create("telegram:123");
        session.add_message("user", "Hello");

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.key, "telegram:123");
    }

    #[test]
    fn test_save_and_load_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Create and modify session
        let session = manager.get_or_create("test:456");
        session.add_message("user", "Test message");
        let key = session.key.clone();

        // Save the session
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Clear cache and reload
        manager.cache.clear();
        let session = manager.get_or_create("test:456");

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Test message");
    }

    #[test]
    fn test_archive_and_reset_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Create and modify session
        let session = manager.get_or_create("archive:789");
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
        let new_session = manager.get_or_create("archive:789");
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

        let session = manager.get_or_create("gui:chat-1");
        session.add_message("user", "Hello");
        let key = session.key.clone();
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Session is in cache; get_or_load should return it
        let loaded = manager.get_or_load("gui:chat-1");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().key, "gui:chat-1");
        assert_eq!(loaded.unwrap().messages.len(), 1);
    }

    #[test]
    fn test_get_or_load_disk_exists_cache_miss() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Create and save session
        let session = manager.get_or_create("gui:chat-2");
        session.add_message("user", "From disk");
        let key = session.key.clone();
        manager.save(&manager.cache.get(&key).unwrap()).unwrap();

        // Clear cache to simulate "not loaded this run"
        manager.cache.clear();

        // get_or_load should load from disk
        let loaded = manager.get_or_load("gui:chat-2");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().key, "gui:chat-2");
        assert_eq!(loaded.unwrap().messages[0].content, "From disk");
    }

    #[test]
    fn test_get_or_load_disk_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        // Session never created; no file on disk
        let loaded = manager.get_or_load("gui:nonexistent");
        assert!(loaded.is_none());
    }
}
