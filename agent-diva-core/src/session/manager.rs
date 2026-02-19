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
#[derive(Debug, Clone)]
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
}
