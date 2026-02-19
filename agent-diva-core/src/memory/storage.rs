//! Memory data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Long-term memory storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Memory {
    /// Memory content in Markdown format
    pub content: String,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Version for conflict detection
    pub version: u64,
}

impl Memory {
    /// Create new empty memory
    pub fn new() -> Self {
        Self {
            content: String::new(),
            updated_at: Utc::now(),
            version: 1,
        }
    }

    /// Create memory with initial content
    pub fn with_content(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            updated_at: Utc::now(),
            version: 1,
        }
    }

    /// Update the memory content
    pub fn update(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Append content to memory
    pub fn append(&mut self, content: impl AsRef<str>) {
        if !self.content.is_empty() {
            self.content.push('\n');
        }
        self.content.push_str(content.as_ref());
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

/// Daily note entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyNote {
    /// Date string (YYYY-MM-DD)
    pub date: String,
    /// Note content in Markdown format
    pub content: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl DailyNote {
    /// Create a new daily note for today
    pub fn today() -> Self {
        let now = Utc::now();
        Self {
            date: now.format("%Y-%m-%d").to_string(),
            content: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a daily note for a specific date
    pub fn for_date(date: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            date: date.into(),
            content: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the note content
    pub fn update(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.updated_at = Utc::now();
    }

    /// Append content to the note
    pub fn append(&mut self, content: impl AsRef<str>) {
        if !self.content.is_empty() {
            self.content.push('\n');
        }
        self.content.push_str(content.as_ref());
        self.updated_at = Utc::now();
    }

    /// Get the filename for this note
    pub fn filename(&self) -> String {
        format!("{}.md", self.date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = Memory::new();
        assert!(memory.content.is_empty());
        assert_eq!(memory.version, 1);
    }

    #[test]
    fn test_memory_with_content() {
        let memory = Memory::with_content("Test content");
        assert_eq!(memory.content, "Test content");
    }

    #[test]
    fn test_memory_update() {
        let mut memory = Memory::new();
        memory.update("New content");
        assert_eq!(memory.content, "New content");
        assert_eq!(memory.version, 2);
    }

    #[test]
    fn test_memory_append() {
        let mut memory = Memory::with_content("Line 1");
        memory.append("Line 2");
        assert_eq!(memory.content, "Line 1\nLine 2");
        assert_eq!(memory.version, 2);
    }

    #[test]
    fn test_daily_note_today() {
        let note = DailyNote::today();
        assert!(!note.date.is_empty());
        assert!(note.content.is_empty());
    }

    #[test]
    fn test_daily_note_filename() {
        let note = DailyNote::for_date("2024-01-15");
        assert_eq!(note.filename(), "2024-01-15.md");
    }
}
