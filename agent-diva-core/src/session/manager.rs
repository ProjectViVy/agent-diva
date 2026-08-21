//! Session manager for handling multiple sessions

use super::store::Session;
use super::store::{
    SESSION_META_CONVERSATION_TITLE, SESSION_META_PINNED, SESSION_META_TITLE_GENERATED,
    SESSION_META_TITLE_MANUALLY_SET,
};
use super::{search::search_sessions_in_dir, SessionSearchQuery, SessionSearchResponse};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
            let session = self.load(key)?;
            self.cache.insert(key.to_string(), session);
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
        let mut title: Option<String> = None;
        let mut last_consolidated: usize = 0;
        let mut canonical_checkpoint = None;

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
                    title = value.get("title").and_then(|v| {
                        if v.is_null() {
                            None
                        } else {
                            v.as_str().map(|s| s.to_string())
                        }
                    });
                    last_consolidated = value
                        .get("last_consolidated")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    canonical_checkpoint = value
                        .get("canonical_checkpoint")
                        .and_then(|value| serde_json::from_value(value.clone()).ok());
                } else if let Ok(msg) = serde_json::from_value::<super::store::ChatMessage>(value) {
                    messages.push(msg);
                }
            }
        }

        let mut session = Session {
            key: key.to_string(),
            messages,
            created_at: created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: chrono::Utc::now(),
            metadata,
            title,
            last_consolidated,
            canonical_checkpoint,
        };
        if let Some(title) = session.conversation_title() {
            session.title = Some(title);
        }
        Some(session)
    }

    /// Save a session to disk
    pub fn save(&self, session: &Session) -> crate::Result<()> {
        self.save_atomic(session, |_| Ok(()))
    }

    fn save_atomic<F>(&self, session: &Session, before_rename: F) -> crate::Result<()>
    where
        F: FnOnce(&Path) -> io::Result<()>,
    {
        fs::create_dir_all(&self.sessions_dir)?;
        let path = self.session_path(&session.key);

        let mut lines = Vec::new();

        // Write metadata
        let mut metadata_map = session.metadata.as_object().cloned().unwrap_or_default();
        match session.conversation_title() {
            Some(title) => {
                metadata_map.insert(
                    SESSION_META_CONVERSATION_TITLE.to_string(),
                    serde_json::Value::String(title),
                );
            }
            None => {
                metadata_map.remove(SESSION_META_CONVERSATION_TITLE);
            }
        }
        metadata_map.insert(
            SESSION_META_TITLE_GENERATED.to_string(),
            serde_json::Value::Bool(session.title_generated()),
        );
        metadata_map.insert(
            SESSION_META_TITLE_MANUALLY_SET.to_string(),
            serde_json::Value::Bool(session.title_manually_set()),
        );
        metadata_map.insert(
            SESSION_META_PINNED.to_string(),
            serde_json::Value::Bool(session.pinned()),
        );

        let metadata = serde_json::json!({
            "_type": "metadata",
            "key": session.key,
            "created_at": session.created_at.to_rfc3339(),
            "updated_at": session.updated_at.to_rfc3339(),
            "metadata": metadata_map,
            "title": session.conversation_title(),
            "last_consolidated": session.last_consolidated,
            "canonical_checkpoint": session.canonical_checkpoint,
        });
        lines.push(serde_json::to_string(&metadata)?);

        // Write messages
        for msg in &session.messages {
            lines.push(serde_json::to_string(msg)?);
        }

        let content = lines.join("\n");
        atomic_replace(&path, content.as_bytes(), before_rename)?;
        Ok(())
    }

    #[cfg(test)]
    fn save_with_hook<F>(&self, session: &Session, before_rename: F) -> crate::Result<()>
    where
        F: FnOnce(&Path) -> io::Result<()>,
    {
        self.save_atomic(session, before_rename)
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
                if entry.path().extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                    continue;
                }

                if let Some(session) = session_summary_from_file(&entry.path()) {
                    sessions.push(session);
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    /// Search session JSONL files without promoting matches into runtime authority.
    pub fn search(&self, query: SessionSearchQuery) -> crate::Result<SessionSearchResponse> {
        search_sessions_in_dir(&self.sessions_dir, query)
    }

    /// Get the file path for a session
    fn session_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace([':', '/', '\\'], "_");
        self.sessions_dir.join(format!("{}.jsonl", safe_key))
    }
}

fn atomic_replace<F>(path: &Path, bytes: &[u8], before_rename: F) -> crate::Result<()>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target path has no parent"))?;
    fs::create_dir_all(parent)?;

    let temp_path = temp_path_for(path);
    let write_result = (|| -> crate::Result<()> {
        write_temp_file(&temp_path, bytes)?;
        before_rename(&temp_path)?;
        fs::rename(&temp_path, path)?;
        sync_parent_dir(parent);
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result
}

fn write_temp_file(path: &Path, bytes: &[u8]) -> crate::Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sync_parent_dir(parent: &Path) {
    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("session-write");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nanos + u128::from(counter)
    ))
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
    /// Session title
    #[serde(default)]
    pub title: Option<String>,
    /// Last visible message preview
    #[serde(default)]
    pub last_message: Option<String>,
    /// Total visible message count
    #[serde(default)]
    pub message_count: usize,
    /// Whether the current title was generated automatically
    #[serde(default)]
    pub title_generated: bool,
    /// Whether the title was set manually
    #[serde(default)]
    pub title_manually_set: bool,
    /// Whether the session is pinned
    #[serde(default)]
    pub pinned: bool,
}

fn session_summary_from_file(path: &Path) -> Option<SessionInfo> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut metadata = serde_json::Map::new();
    let mut created_at = None;
    let mut updated_at = None;
    let mut legacy_title = None;
    let mut key = None;
    let mut last_message = None;
    let mut message_count = 0usize;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        if value.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
            key = value
                .get("key")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or(key);
            created_at = value
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or(created_at);
            updated_at = value
                .get("updated_at")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or(updated_at);
            legacy_title = value
                .get("title")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or(legacy_title);
            metadata = value
                .get("metadata")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            continue;
        }

        let role = value
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let content = value
            .get("content")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or_default();
        if !matches!(role, "user" | "assistant" | "tool") || content.is_empty() {
            continue;
        }
        message_count += 1;
        last_message = Some(compact_preview(content, 120));
    }

    let key = key.or_else(|| {
        path.file_stem()
            .and_then(|name| name.to_str())
            .map(|name| name.replace('_', ":"))
    })?;
    let title = metadata
        .get(SESSION_META_CONVERSATION_TITLE)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            legacy_title
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });

    Some(SessionInfo {
        key,
        created_at,
        updated_at,
        path: path.to_string_lossy().to_string(),
        title,
        last_message,
        message_count,
        title_generated: metadata
            .get(SESSION_META_TITLE_GENERATED)
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        title_manually_set: metadata
            .get(SESSION_META_TITLE_MANUALLY_SET)
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        pinned: metadata
            .get(SESSION_META_PINNED)
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

fn compact_preview(content: &str, max_chars: usize) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(max_chars).collect::<String>();
    if normalized.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
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
        manager
            .save_with_hook(manager.cache.get(&key).unwrap(), |temp_path| {
                assert_eq!(
                    temp_path.parent(),
                    Some(temp_dir.path().join("sessions").as_path())
                );
                Ok(())
            })
            .unwrap();

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
        manager.save(manager.cache.get(&key).unwrap()).unwrap();

        // Archive it
        let archived = manager.archive_and_reset(&key).unwrap();
        assert!(archived);

        // Check it's removed from cache
        assert!(!manager.cache.contains_key(&key));

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
        manager.save(manager.cache.get(&key).unwrap()).unwrap();

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
        manager.save(manager.cache.get(&key).unwrap()).unwrap();

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

    #[test]
    fn test_replace_existing_session_keeps_latest_content() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("replace:1");
        session.add_message("user", "first");
        let key = session.key.clone();
        let snapshot = manager.cache.get(&key).unwrap().clone();
        manager.save(&snapshot).unwrap();

        let mut updated = snapshot.clone();
        updated.add_message("assistant", "second");
        manager.save(&updated).unwrap();

        manager.cache.clear();
        let loaded = manager.get_or_load("replace:1").unwrap();
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].content, "first");
        assert_eq!(loaded.messages[1].content, "second");
    }

    #[test]
    fn test_failed_atomic_save_preserves_previous_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("atomic:fail");
        session.add_message("user", "before");
        let key = session.key.clone();
        let snapshot = manager.cache.get(&key).unwrap().clone();
        manager.save(&snapshot).unwrap();

        let mut updated = snapshot.clone();
        updated.add_message("assistant", "after");

        let err = manager
            .save_with_hook(&updated, |temp_path| {
                assert_eq!(
                    temp_path.parent(),
                    Some(temp_dir.path().join("sessions").as_path())
                );
                Err(io::Error::other("rename blocked"))
            })
            .unwrap_err();
        assert!(matches!(err, crate::Error::Io(_)));

        let final_path = temp_dir.path().join("sessions").join("atomic_fail.jsonl");
        let content = std::fs::read_to_string(&final_path).unwrap();
        assert!(content.contains("\"before\""));
        assert!(!content.contains("\"after\""));

        let temp_entries: Vec<_> = std::fs::read_dir(temp_dir.path().join("sessions"))
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(temp_entries.is_empty());
    }

    #[test]
    fn test_session_title_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("title:test");
        session.set_conversation_title(Some("Hello World".to_string()));
        session.set_title_generated(true);
        let key = session.key.clone();
        manager.save(manager.cache.get(&key).unwrap()).unwrap();

        manager.cache.clear();
        let loaded = manager.get_or_load("title:test").unwrap();
        assert_eq!(loaded.title, Some("Hello World".to_string()));

        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, Some("Hello World".to_string()));
        assert!(sessions[0].title_generated);
    }

    #[test]
    fn test_session_title_backwards_compat() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let path = manager.session_path("legacy:title");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let metadata_line = serde_json::json!({
            "_type": "metadata",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "metadata": {},
            "last_consolidated": 0,
            "canonical_checkpoint": null,
        });
        fs::write(&path, format!("{}\n", metadata_line)).unwrap();

        let loaded = manager.get_or_load("legacy:title");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().title, None);

        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, None);
    }

    #[test]
    fn list_sessions_includes_last_message_and_flags() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("gui:rich");
        session.add_message("user", "First user message");
        session.add_message("assistant", "Assistant reply");
        session.set_conversation_title(Some("Rich Session".to_string()));
        session.set_title_generated(true);
        session.set_title_manually_set(false);
        session.set_pinned(true);
        let key = session.key.clone();
        let snapshot = manager.cache.get(&key).unwrap().clone();
        manager.save(&snapshot).unwrap();

        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 1);
        let info = &sessions[0];
        assert_eq!(info.title.as_deref(), Some("Rich Session"));
        assert_eq!(info.last_message.as_deref(), Some("Assistant reply"));
        assert_eq!(info.message_count, 2);
        assert!(info.title_generated);
        assert!(!info.title_manually_set);
        assert!(info.pinned);
    }

    #[test]
    fn list_sessions_handles_empty_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path());

        let session = manager.get_or_create("gui:empty");
        let key = session.key.clone();
        let snapshot = manager.cache.get(&key).unwrap().clone();
        manager.save(&snapshot).unwrap();

        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 1);
        let info = &sessions[0];
        assert_eq!(info.message_count, 0);
        assert_eq!(info.last_message, None);
        assert_eq!(info.title, None);
    }
}
