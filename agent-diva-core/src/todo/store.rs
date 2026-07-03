//! JSONL append-only store for runtime todo items

use super::types::{TodoItem, TodoStatus};
use chrono::{Datelike, Duration, Utc};
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

    /// Archive completed todos whose `updated_at` is older than `days_old` days.
    /// Returns the number of items archived.
    pub async fn archive_completed(&self, days_old: u64) -> std::io::Result<usize> {
        let items = self.read_all()?;
        let cutoff = Utc::now() - Duration::days(days_old as i64);

        let (to_archive, to_keep): (Vec<TodoItem>, Vec<TodoItem>) = items
            .into_iter()
            .partition(|item| item.status == TodoStatus::Completed && item.updated_at < cutoff);

        if to_archive.is_empty() {
            return Ok(0);
        }

        // Write archived items to the archive file
        let archive_path = self.archive_path_for_date(Utc::now());
        let mut archive_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&archive_path)?;

        for item in &to_archive {
            let mut line = serde_json::to_string(item)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            line.push('\n');
            archive_file.write_all(line.as_bytes())?;
        }
        archive_file.flush()?;

        // Rewrite main file without archived items
        self.rewrite_all(&to_keep)?;

        Ok(to_archive.len())
    }

    /// Purge archive files older than `months_old` months.
    /// Returns the number of archive files deleted.
    pub async fn purge_archived(&self, months_old: u64) -> std::io::Result<usize> {
        let cutoff = Utc::now() - Duration::days((months_old * 30) as i64);
        let cutoff_year_month = format!("{:04}-{:02}", cutoff.year(), cutoff.month());

        let store_dir = self
            .path
            .parent()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "invalid store path"))?;

        let mut deleted = 0;
        for entry in std::fs::read_dir(store_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if let Some(year_month) = name
                .strip_prefix("todos.archive.")
                .and_then(|s| s.strip_suffix(".jsonl"))
            {
                if year_month < cutoff_year_month.as_str() {
                    std::fs::remove_file(entry.path())?;
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    /// Generate archive file path for a given date: `{store_dir}/todos.archive.{YYYY-MM}.jsonl`
    fn archive_path_for_date(&self, date: chrono::DateTime<Utc>) -> PathBuf {
        let store_dir = self
            .path
            .parent()
            .expect("store path has a parent directory");
        let year_month = format!("{:04}-{:02}", date.year(), date.month());
        store_dir.join(format!("todos.archive.{year_month}.jsonl"))
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
    fn sample_item(title: &str, source: &str) -> TodoItem {
        TodoItem::new(title, source)
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let (_dir, store) = setup();
        let item = sample_item("Write tests", "user");
        let created = store.create(item).await.expect("create");
        let items = store.list().await.expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, created.id);
        assert_eq!(items[0].title, "Write tests");
    }

    #[tokio::test]
    async fn test_get_found() {
        let (_dir, store) = setup();
        let item = sample_item("Find me", "agent");
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
            .create(sample_item("Task 1", "user"))
            .await
            .expect("create");
        store
            .create(sample_item("Task 2", "cron"))
            .await
            .expect("create");
        store
            .create(sample_item("Task 3", "agent"))
            .await
            .expect("create");
        let items = store.list().await.expect("list");
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_update_status_pending_to_active() {
        let (_dir, store) = setup();
        let created = store
            .create(sample_item("Status change", "user"))
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
        let mut item = sample_item("Already done", "user");
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
        let mut item = sample_item("Never mind", "user");
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
            .create(sample_item("Line 1", "user"))
            .await
            .expect("create");
        store
            .create(sample_item("Line 2", "cron"))
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

    #[tokio::test]
    async fn test_archive_completed_moves_old_items() {
        let dir = TempDir::new().expect("tempdir");
        let store = JsonlTodoStore::new(dir.path()).expect("store");

        // Create an completed todo with old updated_at
        let mut old_item = sample_item("Old done task", "user");
        old_item.status = TodoStatus::Completed;
        old_item.updated_at = Utc::now() - Duration::days(60);
        store.create(old_item.clone()).await.expect("create");

        // Create a recent completed todo
        let mut recent_item = sample_item("Recent done task", "user");
        recent_item.status = TodoStatus::Completed;
        recent_item.updated_at = Utc::now() - Duration::days(5);
        store.create(recent_item.clone()).await.expect("create");

        // Create a pending todo (should not be archived)
        let pending_item = sample_item("Pending task", "user");
        store.create(pending_item.clone()).await.expect("create");

        // Archive items older than 30 days
        let archived = store.archive_completed(30).await.expect("archive");
        assert_eq!(archived, 1);

        // Main file should only have recent and pending items
        let remaining = store.list().await.expect("list");
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|i| i.id == recent_item.id));
        assert!(remaining.iter().any(|i| i.id == pending_item.id));

        // Archive file should have the old item
        let archive_path = dir.path().join(format!("todos.archive.{:04}-{:02}.jsonl", Utc::now().year(), Utc::now().month()));
        let archive_content = std::fs::read_to_string(&archive_path).expect("read archive");
        assert!(archive_content.contains(&old_item.id));
    }

    #[tokio::test]
    async fn test_archive_completed_no_match() {
        let dir = TempDir::new().expect("tempdir");
        let store = JsonlTodoStore::new(dir.path()).expect("store");

        // Create a recent completed todo
        let mut item = sample_item("Recent done", "user");
        item.status = TodoStatus::Completed;
        item.updated_at = Utc::now() - Duration::days(5);
        store.create(item).await.expect("create");

        // Archive items older than 30 days — nothing should match
        let archived = store.archive_completed(30).await.expect("archive");
        assert_eq!(archived, 0);

        // Main file should still have the item
        let remaining = store.list().await.expect("list");
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test]
    async fn test_purge_archived_deletes_old_files() {
        let dir = TempDir::new().expect("tempdir");
        let store = JsonlTodoStore::new(dir.path()).expect("store");

        // Create an old archive file (simulate)
        let old_archive = dir.path().join("todos.archive.2024-01.jsonl");
        std::fs::write(&old_archive, "{}\n").expect("write old archive");

        // Create a recent archive file
        let now = Utc::now();
        let recent_archive = dir.path().join(format!("todos.archive.{:04}-{:02}.jsonl", now.year(), now.month()));
        std::fs::write(&recent_archive, "{}\n").expect("write recent archive");

        // Purge archives older than 6 months
        let deleted = store.purge_archived(6).await.expect("purge");
        assert_eq!(deleted, 1);

        // Old archive should be deleted
        assert!(!old_archive.exists());
        // Recent archive should still exist
        assert!(recent_archive.exists());
    }

    #[tokio::test]
    async fn test_purge_archived_no_old_files() {
        let dir = TempDir::new().expect("tempdir");
        let store = JsonlTodoStore::new(dir.path()).expect("store");

        // Create a recent archive file
        let now = Utc::now();
        let recent_archive = dir.path().join(format!("todos.archive.{:04}-{:02}.jsonl", now.year(), now.month()));
        std::fs::write(&recent_archive, "{}\n").expect("write recent archive");

        // Purge archives older than 6 months — nothing should match
        let deleted = store.purge_archived(6).await.expect("purge");
        assert_eq!(deleted, 0);

        // Recent archive should still exist
        assert!(recent_archive.exists());
    }
}
