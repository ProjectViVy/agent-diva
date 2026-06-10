//! Memory manager for handling long-term memory

use super::provider::{
    MemoryProvider, PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest,
    SessionEndResponse, SessionEndStatus, StartupInjectionShape, SyncTurnRequest, SyncTurnResponse,
    SyncTurnStatus, SystemPromptBlock, SystemPromptRequest, SystemPromptResponse,
};
use super::storage::{DailyNote, Memory};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Manages long-term memory storage
#[derive(Debug)]
pub struct MemoryManager {
    /// Workspace directory
    _workspace: PathBuf,
    /// Memory file path
    memory_path: PathBuf,
    /// Daily notes directory
    notes_dir: PathBuf,
    /// History file path
    history_path: PathBuf,
    /// Session IDs whose shutdown hook has already been handled.
    handled_session_end_ids: Mutex<HashSet<String>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new<P: AsRef<Path>>(workspace: P) -> Self {
        let workspace = workspace.as_ref().to_path_buf();
        let memory_path = workspace.join("memory").join("MEMORY.md");
        let history_path = workspace.join("memory").join("HISTORY.md");
        let notes_dir = workspace.join("memory");

        Self {
            _workspace: workspace,
            memory_path,
            notes_dir,
            history_path,
            handled_session_end_ids: Mutex::new(HashSet::new()),
        }
    }

    /// Load the long-term memory
    pub fn load_memory(&self) -> Memory {
        if self.memory_path.exists() {
            match std::fs::read_to_string(&self.memory_path) {
                Ok(content) => Memory::with_content(content),
                Err(_) => Memory::new(),
            }
        } else {
            Memory::new()
        }
    }

    /// Save the long-term memory
    pub fn save_memory(&self, memory: &Memory) -> crate::Result<()> {
        if let Some(parent) = self.memory_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::utils::atomic_write(&self.memory_path, memory.content.as_bytes())
    }

    /// Load history entries from `HISTORY.md`
    pub fn load_history(&self) -> String {
        if self.history_path.exists() {
            std::fs::read_to_string(&self.history_path).unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// Append an entry to `HISTORY.md`
    pub fn append_history(&self, entry: &str) -> crate::Result<()> {
        if entry.trim().is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.history_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut content = self.load_history();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(entry.trim_end());
        content.push_str("\n\n");
        crate::utils::atomic_write(&self.history_path, content.as_bytes())
    }

    /// Load a daily note
    pub fn load_daily_note(&self, date: impl AsRef<str>) -> DailyNote {
        let date = date.as_ref();
        let path = self.notes_dir.join(format!("{}.md", date));

        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let mut note = DailyNote::for_date(date);
                    note.content = content;
                    note
                }
                Err(_) => DailyNote::for_date(date),
            }
        } else {
            DailyNote::for_date(date)
        }
    }

    /// Load today's note
    pub fn load_today_note(&self) -> DailyNote {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.load_daily_note(&today)
    }

    /// Save a daily note
    pub fn save_daily_note(&self, note: &DailyNote) -> crate::Result<()> {
        std::fs::create_dir_all(&self.notes_dir)?;
        let path = self.notes_dir.join(note.filename());
        crate::utils::atomic_write(&path, note.content.as_bytes())
    }

    /// List all daily notes
    pub fn list_notes(&self) -> Vec<String> {
        let mut notes = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.notes_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".md") && name != "MEMORY.md" {
                        let date = name.trim_end_matches(".md").to_string();
                        notes.push(date);
                    }
                }
            }
        }

        notes.sort_by(|a, b| b.cmp(a)); // Newest first
        notes
    }

    /// Get the memory directory path
    pub fn memory_dir(&self) -> &Path {
        &self.notes_dir
    }

    /// Append content to today's daily note
    pub fn append_today(&self, content: &str) -> crate::Result<()> {
        let mut note = self.load_today_note();

        if note.content.is_empty() {
            // Add header for new day
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            note.content = format!("# {}\n\n{}", today, content);
        } else {
            // Append to existing content
            note.content.push('\n');
            note.content.push_str(content);
        }

        self.save_daily_note(&note)
    }

    /// Get memories from the last N days
    pub fn get_recent_memories(&self, days: usize) -> String {
        use chrono::Duration;

        let mut memories = Vec::new();
        let today = chrono::Local::now().date_naive();

        for i in 0..days {
            let date = today - Duration::days(i as i64);
            let date_str = date.format("%Y-%m-%d").to_string();
            let note = self.load_daily_note(&date_str);

            if !note.content.is_empty() {
                memories.push(note.content);
            }
        }

        memories.join("\n\n---\n\n")
    }

    /// List all memory files sorted by date (newest first)
    pub fn list_memory_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.notes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Match pattern YYYY-MM-DD.md
                    if name.len() == 13 && name.ends_with(".md") && name != "MEMORY.md" {
                        let date_part = &name[..10];
                        // Basic validation: check if it looks like a date
                        if date_part.chars().filter(|c| *c == '-').count() == 2 {
                            files.push(path);
                        }
                    }
                }
            }
        }

        // Sort by filename (which is the date) in reverse order
        files.sort_by(|a, b| b.cmp(a));
        files
    }

    /// Get memory context for the agent.
    /// The redesigned memory model injects only long-term memory into prompts.
    pub fn get_memory_context(&self) -> String {
        let memory = self.load_memory();
        if memory.content.is_empty() {
            String::new()
        } else {
            format!("## Long-term Memory\n{}", memory.content)
        }
    }
}

#[async_trait::async_trait]
impl MemoryProvider for MemoryManager {
    fn system_prompt_block(
        &self,
        _request: &SystemPromptRequest,
    ) -> crate::Result<SystemPromptResponse> {
        let context = self.get_memory_context();
        if context.is_empty() {
            Ok(SystemPromptResponse::degraded(
                "startup continuity unavailable; no long-term memory available",
            ))
        } else {
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown: context,
            }))
        }
    }

    async fn prefetch(&self, request: PrefetchRequest) -> crate::Result<PrefetchResponse> {
        if request.intent.trim().is_empty() {
            return Ok(PrefetchResponse {
                status: PrefetchStatus::SkippedNoIntent,
                prompt_block: None,
            });
        }

        Ok(PrefetchResponse {
            status: PrefetchStatus::Failed {
                reason: format!(
                    "prefetch recall is unavailable in the default MemoryManager for intent '{}'",
                    request.intent.trim()
                ),
            },
            prompt_block: None,
        })
    }

    async fn sync_turn(&self, request: SyncTurnRequest) -> crate::Result<SyncTurnResponse> {
        let mut persisted = false;

        if let Some(memory_update) = request.memory_update_markdown.as_deref() {
            if !memory_update.trim().is_empty() {
                let memory = Memory::with_content(memory_update);
                if let Err(err) = self.save_memory(&memory) {
                    return Ok(SyncTurnResponse {
                        status: SyncTurnStatus::Failed {
                            reason: format!("failed to persist MEMORY.md: {err}"),
                        },
                    });
                }
                persisted = true;
            }
        }

        if let Some(history_entry) = request.history_entry.as_deref() {
            if !history_entry.trim().is_empty() {
                if let Err(err) = self.append_history(history_entry) {
                    return Ok(SyncTurnResponse {
                        status: SyncTurnStatus::Failed {
                            reason: format!("failed to append HISTORY.md: {err}"),
                        },
                    });
                }
                persisted = true;
            }
        }

        Ok(SyncTurnResponse {
            status: if persisted {
                SyncTurnStatus::Persisted
            } else {
                SyncTurnStatus::Noop
            },
        })
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> crate::Result<SessionEndResponse> {
        if let Some(session_id) = request.session_id {
            let mut handled = self.handled_session_end_ids.lock();
            if !handled.insert(session_id) {
                return Ok(SessionEndResponse {
                    status: SessionEndStatus::AlreadyHandled,
                });
            }
        }

        Ok(SessionEndResponse {
            status: SessionEndStatus::Noop,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        MemoryProvider, PrefetchRequest, PrefetchStatus, SessionEndRequest, SessionEndStatus,
        StartupStatus, SyncTurnRequest, SyncTurnStatus, SystemPromptRequest,
    };
    use tempfile::TempDir;

    #[test]
    fn test_append_history() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());
        manager
            .append_history("[2026-02-12 09:00] Added memory event")
            .unwrap();

        let history = manager.load_history();
        assert!(history.contains("Added memory event"));
    }

    #[tokio::test]
    async fn test_memory_provider_system_prompt_block_reads_memory() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());
        manager
            .save_memory(&Memory::with_content("Long term provider context"))
            .unwrap();

        let response = manager
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap();

        assert_eq!(response.status, StartupStatus::Ready);

        let block = response
            .prompt_block
            .expect("memory-backed provider should emit a prompt block");

        assert!(block.markdown.contains("## Long-term Memory"));
        assert!(block.markdown.contains("Long term provider context"));
        assert_eq!(
            block.shape,
            crate::memory::StartupInjectionShape::CompactRenderedMarkdown
        );
    }

    #[tokio::test]
    async fn test_memory_provider_sync_turn_persists_memory_and_history() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let result = manager
            .sync_turn(SyncTurnRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                memory_update_markdown: Some("Updated memory from sync_turn".to_string()),
                history_entry: Some("[2026-05-08 10:00 UTC] synchronized turn".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result.status, SyncTurnStatus::Persisted);
        assert_eq!(
            manager.load_memory().content,
            "Updated memory from sync_turn"
        );
        assert!(manager.load_history().contains("synchronized turn"));
    }

    #[tokio::test]
    async fn test_memory_provider_session_end_is_noop_by_default() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let response = manager
            .on_session_end(SessionEndRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                session_id: Some("session-1".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, SessionEndStatus::Noop);
    }

    #[tokio::test]
    async fn test_memory_provider_returns_degraded_startup_when_no_context_is_available() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let response = manager
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap();

        match response.status {
            StartupStatus::Degraded {
                reason,
                last_usable_wakeup,
            } => {
                assert!(reason.contains("startup continuity unavailable"));
                assert!(last_usable_wakeup.is_none());
            }
            other => panic!("expected degraded startup, got {other:?}"),
        }

        let block = response
            .prompt_block
            .expect("degraded startup should still provide explicit startup text");
        assert!(block.markdown.contains("status: degraded"));
        assert!(block.markdown.contains("last_usable_wakeup: omitted"));
    }

    #[tokio::test]
    async fn test_memory_provider_prefetch_distinguishes_no_intent_from_failed_recall() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let skipped = manager
            .prefetch(PrefetchRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                intent: "   ".to_string(),
                current_room: None,
                user_message: Some("help".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(skipped.status, PrefetchStatus::SkippedNoIntent);
        assert!(skipped.prompt_block.is_none());

        let failed = manager
            .prefetch(PrefetchRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                intent: "recall-project-status".to_string(),
                current_room: Some("roadmap".to_string()),
                user_message: Some("what changed?".to_string()),
            })
            .await
            .unwrap();
        match failed.status {
            PrefetchStatus::Failed { reason } => {
                assert!(reason.contains("prefetch recall is unavailable"));
            }
            other => panic!("expected failed recall, got {other:?}"),
        }
        assert!(failed.prompt_block.is_none());
    }

    #[tokio::test]
    async fn test_memory_provider_sync_turn_failure_is_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(workspace.join("memory")).unwrap();
        std::fs::write(workspace.join("memory").join("MEMORY.md"), "locked").unwrap();
        std::fs::remove_file(workspace.join("memory").join("MEMORY.md")).unwrap();
        std::fs::create_dir_all(workspace.join("memory").join("MEMORY.md")).unwrap();

        let manager = MemoryManager::new(&workspace);
        let result = manager
            .sync_turn(SyncTurnRequest {
                workspace_root: workspace,
                memory_update_markdown: Some("cannot persist".to_string()),
                history_entry: None,
            })
            .await
            .unwrap();

        match result.status {
            SyncTurnStatus::Failed { reason } => {
                assert!(reason.contains("failed to persist MEMORY.md"));
            }
            other => panic!("expected sync failure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_memory_provider_session_end_is_idempotent_for_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let first = manager
            .on_session_end(SessionEndRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                session_id: Some("session-dup".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(first.status, SessionEndStatus::Noop);

        let duplicate = manager
            .on_session_end(SessionEndRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                session_id: Some("session-dup".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(duplicate.status, SessionEndStatus::AlreadyHandled);
    }
}
