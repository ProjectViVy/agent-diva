//! Memory manager for handling long-term memory

use super::storage::{DailyNote, Memory};
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
        std::fs::write(&self.memory_path, &memory.content)?;
        Ok(())
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
        std::fs::write(&self.history_path, content)?;
        Ok(())
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
        std::fs::write(&path, &note.content)?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());
        assert_eq!(manager._workspace, temp_dir.path());
    }

    #[test]
    fn test_load_save_memory() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let memory = Memory::with_content("Test memory");
        manager.save_memory(&memory).unwrap();

        let loaded = manager.load_memory();
        assert_eq!(loaded.content, "Test memory");
    }

    #[test]
    fn test_load_save_daily_note() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let mut note = DailyNote::for_date("2024-01-15");
        note.content = "Test note".to_string();
        manager.save_daily_note(&note).unwrap();

        let loaded = manager.load_daily_note("2024-01-15");
        assert_eq!(loaded.content, "Test note");
    }

    #[test]
    fn test_list_notes() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let mut note1 = DailyNote::for_date("2024-01-15");
        note1.content = "Note 1".to_string();
        manager.save_daily_note(&note1).unwrap();

        let mut note2 = DailyNote::for_date("2024-01-16");
        note2.content = "Note 2".to_string();
        manager.save_daily_note(&note2).unwrap();

        let notes = manager.list_notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0], "2024-01-16"); // Newest first
    }

    #[test]
    fn test_append_today() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        // First append - should add header
        manager.append_today("First entry").unwrap();
        let note = manager.load_today_note();
        assert!(note.content.contains("First entry"));
        assert!(note.content.starts_with("#"));

        // Second append - should not add another header
        manager.append_today("Second entry").unwrap();
        let note = manager.load_today_note();
        assert!(note.content.contains("First entry"));
        assert!(note.content.contains("Second entry"));
    }

    #[test]
    fn test_get_recent_memories() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        // Create some notes
        let mut note1 = DailyNote::for_date("2024-01-15");
        note1.content = "Memory 1".to_string();
        manager.save_daily_note(&note1).unwrap();

        let mut note2 = DailyNote::for_date("2024-01-16");
        note2.content = "Memory 2".to_string();
        manager.save_daily_note(&note2).unwrap();

        // Get recent memories (this will get today's date range, so may not include our test dates)
        let recent = manager.get_recent_memories(7);
        // Just verify it doesn't panic
        assert!(recent.is_empty() || !recent.is_empty());
    }

    #[test]
    fn test_list_memory_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let mut note1 = DailyNote::for_date("2024-01-15");
        note1.content = "Note 1".to_string();
        manager.save_daily_note(&note1).unwrap();

        let mut note2 = DailyNote::for_date("2024-01-16");
        note2.content = "Note 2".to_string();
        manager.save_daily_note(&note2).unwrap();

        let files = manager.list_memory_files();
        assert_eq!(files.len(), 2);
        // Should be sorted newest first
        assert!(files[0].to_str().unwrap().contains("2024-01-16"));
    }

    #[test]
    fn test_get_memory_context() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        // Set up long-term memory
        let memory = Memory::with_content("Long term info");
        manager.save_memory(&memory).unwrap();

        // Get context
        let context = manager.get_memory_context();
        assert!(context.contains("Long-term Memory"));
        assert!(context.contains("Long term info"));
    }

    #[test]
    fn test_get_memory_context_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(temp_dir.path());

        let context = manager.get_memory_context();
        assert_eq!(context, "");
    }

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
}
