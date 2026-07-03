//! JSONL append-only store for runtime todo items

use super::types::{TodoItem, TodoStatus};
#[cfg(test)]
use super::types::TodoSource;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Append-only JSONL store for todo items
pub struct JsonlTodoStore {
    path: PathBuf,
}

impl JsonlTodoStore {
    /// Create a new store rooted at `data_root`.
    /// The JSONL file will be at `{data_root}/todos.jsonl`.
    /// Creates the file if it does not exist.
    pub fn new(data_root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(data_root)?;
        let path = data_root.join("todos.jsonl");
        // Ensure the file exists
        OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path })
    }

    /// Append a new todo item to the store.
    /// Returns the item as stored (unchanged).
    pub async fn create(&self, item: TodoItem) -> std::io::Result<TodoItem> {
        let mut line = serde_json::to_string(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');

        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(item)
    }

    /// Get a todo item by id.
    /// Returns None if not found.
    pub async fn get(&self, id: &str) -> std::io::Result<Option<TodoItem>> {
        let items = self.read_all()?;
        Ok(items.into_iter().find(|item| item.id == id))
    }

    /// List all todo items.
    pub async fn list(&self) -> std::io::Result<Vec<TodoItem>> {
        self.read_all()
    }

    /// Update the status of a todo item by id.
    /// Terminal states (Completed, Cancelled) are no-op — returns the current item unchanged.
    /// For non-terminal states, the file is rewritten with the updated item.
    /// Returns None if the item is not found.
    pub async fn update_status(
        &self,
        id: &str,
        new_status: TodoStatus,
    ) -> std::io::Result<Option<TodoItem>> {
        let mut items = self.read_all()?;
        let Some(item) = items.iter_mut().find(|i| i.id == id) else {
            return Ok(None);
        };

        // Terminal states are immutable
        if item.status.is_terminal() {
            let unchanged = item.clone();
            return Ok(Some(unchanged));
        }

        item.status = new_status;
        item.updated_at = Utc::now();
        let updated = item.clone();
        self.rewrite_all(&items)?;
        Ok(Some(updated))
    }

    // -- private helpers --

    /// Read all items from the JSONL file, skipping malformed lines.
    fn read_all(&self) -> std::io::Result<Vec<TodoItem>> {
        let file = std::fs::File::open(&self.path)?;
        let reader = std::io::BufReader::new(file);
        let mut items = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(item) = serde_json::from_str::<TodoItem>(trimmed) {
                items.push(item);
            }
            // Silently skip malformed lines
        }
        Ok(items)
    }

    /// Rewrite the entire JSONL file with the given items.
    fn rewrite_all(&self, items: &[TodoItem]) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        for item in items {
            let mut line = serde_json::to_string(item)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            line.push('\n');
            file.write_all(line.as_bytes())?;
        }
        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a store backed by a temp directory
    fn setup() -> (TempDir, JsonlTodoStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = JsonlTodoStore::new(dir.path()).expect("store creation");
        (dir, store)
    }

    /// Helper to create a sample TodoItem
    fn sample_item(title: &str, source: TodoSource) -> TodoItem {
        TodoItem::new(title, source)
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let (_dir, store) = setup();
        let item = sample_item("Write tests", TodoSource::User);
        let created = store.create(item).await.expect("create");
        let items = store.list().await.expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, created.id);
        assert_eq!(items[0].title, "Write tests");
    }

    #[tokio::test]
    async fn test_get_found() {
        let (_dir, store) = setup();
        let item = sample_item("Find me", TodoSource::Agent);
        let created = store.create(item).await.expect("create");
        let found = store.get(&created.id).await.expect("get");
        assert!(found.is_some());
        let found = found.expect("item");
        assert_eq!(found.title, "Find me");
        assert_eq!(found.source, "agent");
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let (_dir, store) = setup();
        let found = store.get("nonexistent-id").await.expect("get");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_multiple() {
        let (_dir, store) = setup();
        store
            .create(sample_item("Task 1", TodoSource::User))
            .await
            .expect("create");
        store
            .create(sample_item("Task 2", TodoSource::Cron))
            .await
            .expect("create");
        store
            .create(sample_item("Task 3", TodoSource::Agent))
            .await
            .expect("create");
        let items = store.list().await.expect("list");
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_update_status_pending_to_active() {
        let (_dir, store) = setup();
        let created = store
            .create(sample_item("Status change", TodoSource::User))
            .await
            .expect("create");
        assert_eq!(created.status, TodoStatus::Pending);

        let updated = store
            .update_status(&created.id, TodoStatus::Active)
            .await
            .expect("update");
        assert!(updated.is_some());
        let updated = updated.expect("item");
        assert_eq!(updated.status, TodoStatus::Active);

        // Verify persistence
        let reloaded = store.get(&created.id).await.expect("get").expect("item");
        assert_eq!(reloaded.status, TodoStatus::Active);
    }

    #[tokio::test]
    async fn test_terminal_state_completed_noop() {
        let (_dir, store) = setup();
        let mut item = sample_item("Already done", TodoSource::User);
        item.status = TodoStatus::Completed;
        let created = store.create(item).await.expect("create");

        let result = store
            .update_status(&created.id, TodoStatus::Active)
            .await
            .expect("update");
        assert!(result.is_some());
        assert_eq!(result.expect("item").status, TodoStatus::Completed);
    }

    #[tokio::test]
    async fn test_terminal_state_cancelled_noop() {
        let (_dir, store) = setup();
        let mut item = sample_item("Never mind", TodoSource::User);
        item.status = TodoStatus::Cancelled;
        let created = store.create(item).await.expect("create");

        let result = store
            .update_status(&created.id, TodoStatus::Active)
            .await
            .expect("update");
        assert!(result.is_some());
        assert_eq!(result.expect("item").status, TodoStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_jsonl_format() {
        let dir = TempDir::new().expect("tempdir");
        let store = JsonlTodoStore::new(dir.path()).expect("store");
        store
            .create(sample_item("Line 1", TodoSource::User))
            .await
            .expect("create");
        store
            .create(sample_item("Line 2", TodoSource::Cron))
            .await
            .expect("create");

        let content = std::fs::read_to_string(dir.path().join("todos.jsonl")).expect("read");
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2);

        // Each line must be valid JSON
        for line in &lines {
            let parsed: serde_json::Value = serde_json::from_str(line).expect("valid json");
            assert!(parsed.is_object());
        }
    }

    #[tokio::test]
    async fn test_update_status_not_found() {
        let (_dir, store) = setup();
        let result = store
            .update_status("ghost-id", TodoStatus::Active)
            .await
            .expect("update");
        assert!(result.is_none());
    }
}
