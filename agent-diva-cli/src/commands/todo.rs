use agent_diva_core::todo::{JsonlTodoStore, TodoItem, TodoStatus};
use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::Path;

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum TodoCommands {
    /// List all todos
    List,
    /// Add a new todo
    Add {
        /// Todo title
        title: String,
    },
    /// Update a todo status
    Update {
        /// Todo ID
        id: String,
        /// New status (open|pending|active|done|completed|cancelled)
        #[arg(long)]
        status: String,
    },
    /// Archive completed todos older than N days
    Archive {
        /// Number of days old to archive
        #[arg(long, default_value = "30")]
        days: u64,
    },
    /// Purge archived todo files older than N months
    Purge {
        /// Number of months old to purge
        #[arg(long, default_value = "6")]
        months: u64,
    },
}

pub async fn run(command: TodoCommands, data_root: &Path) -> Result<()> {
    let store = JsonlTodoStore::new(data_root)
        .with_context(|| format!("Failed to open todo store at {}", data_root.display()))?;

    match command {
        TodoCommands::List => {
            let items = store.list().await.context("Failed to list todos")?;
            if items.is_empty() {
                println!("No todos found.");
            } else {
                println!(" {:<36} {:<12} Title", "ID", "Status");
                println!("{}", "-".repeat(80));
                for item in items {
                    let status = format!("{:?}", item.status).to_lowercase();
                    println!(" {:<36} {:<12} {}", item.id, status, item.title);
                }
            }
        }
        TodoCommands::Add { title } => {
            let item = TodoItem::new(&title, "cli");
            let created = store.create(item).await.context("Failed to create todo")?;
            println!("Created todo {}: {}", created.id, created.title);
        }
        TodoCommands::Update { id, status } => {
            let new_status = parse_status(&status)?;
            let updated = store
                .update_status(&id, new_status)
                .await
                .context("Failed to update todo")?;
            match updated {
                Some(item) => {
                    println!("Updated todo {} to {:?}", item.id, item.status);
                }
                None => {
                    anyhow::bail!("Todo not found: {}", id);
                }
            }
        }
        TodoCommands::Archive { days } => {
            let archived = store
                .archive_completed(days)
                .await
                .context("Failed to archive todos")?;
            println!(
                "Archived {} completed todo(s) older than {} days",
                archived, days
            );
        }
        TodoCommands::Purge { months } => {
            let deleted = store
                .purge_archived(months)
                .await
                .context("Failed to purge archived todos")?;
            println!(
                "Purged {} archive file(s) older than {} months",
                deleted, months
            );
        }
    }

    Ok(())
}

fn parse_status(s: &str) -> Result<TodoStatus> {
    match TodoStatus::parse_update(s) {
        Some(status) => Ok(status),
        None => anyhow::bail!(
            "Invalid status: {}. Use: open, pending, active, done, completed, or cancelled",
            s
        ),
    }
}
